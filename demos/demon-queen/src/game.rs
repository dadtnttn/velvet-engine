//! Versioned JSON story importer, branching runtime and atomic save support.
//!
//! `story/prologue.json` is authoritative. The renderer receives a flat
//! [`SceneView`], while this module owns rooms, characters, conditions,
//! effects, choices, validation and persistence.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

const STORY_FORMAT: &str = "velvet-story";
const STORY_SCHEMA: u32 = 1;
const SAVE_FORMAT: &str = "velvet-story-save";
const SAVE_SCHEMA: u32 = 1;
const GALLERY_FORMAT: &str = "demon-queen-gallery";
const GALLERY_SCHEMA: u32 = 1;
const MAX_CHOICES: usize = 8;
const ROLLBACK_LIMIT: usize = 128;
// Side stories are authored in the same document but are entered from the
// explorable world instead of the released prologue route.  They intentionally
// fall back to the source text until their translated catalogue entries exist.
const SIDE_VN_TAG: &str = "side_vn";
/// Nodes of the prologue the guion one replaced (and of the act it led to): kept for saves made in
/// them, out of the way of every new game (see `with_guiones` and `migrate_state`).
const OLD_PROLOGUE_TAG: &str = "prologo_antiguo";
pub const MANUAL_SLOTS: usize = 12;

pub struct SaveSlot {
    pub saved_at: u64,
    pub scene: Option<SceneView>,
    pub is_world: bool,
    pub thumbnail: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SceneView {
    pub id: String,
    pub room_id: String,
    pub room_name: String,
    pub bg: String,
    pub speaker_id: String,
    pub speaker: String,
    pub text: String,
    pub image_asset: Option<String>,
    /// Layered VN sprites rendered over the room background.  Side VN scenes
    /// use these instead of a full-screen CG so the composition behaves like
    /// the reference VN: background, left/right characters, dialogue chrome.
    pub layers: Vec<SceneLayer>,
    pub choices: Vec<Choice>,
    pub ending: bool,
    pub show_hud: bool,
    pub world_entry: Option<WorldEntry>,
    /// A title card shown as the node starts (a guion's `@titulo`: «PRÓLOGO · 1 | La última ronda»).
    pub title_card: Option<String>,
    /// Keys shown on screen while the node is up (a guion's `@teclas`: «W A S D · caminar | F · usar»).
    pub keys: Option<String>,
    /// The line is a sound (a guion's `*…*`), not narration.
    pub sound: bool,
    /// How the speaker looks saying it (a guion's `**Chiquita (enfadada):**`).
    pub mood: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneLayer {
    pub asset: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntry {
    pub world: String,
    pub spawn: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub fear: i64,
    pub devotion: i64,
    pub dignity: i64,
    pub hunger: i64,
    pub decree: i64,
    pub desire: i64,
    /// «El Relevo»: how much the court suspects the Queen is not herself.
    pub suspicion: i64,
    /// How far Lira and Nemi trust her.
    pub maids_trust: i64,
}

/// Variables that start in the 3D world rather than in the prologue's story file.
const WORLD_ONLY_VARIABLES: &[&str] = &["deseo"];

#[derive(Debug, Clone, Serialize)]
pub struct StorySummary {
    pub id: String,
    pub title: String,
    pub revision: u32,
    pub rooms: usize,
    pub characters: usize,
    pub nodes: usize,
    pub image_briefs: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagePlanEntry {
    pub node: String,
    pub room: String,
    pub speaker: String,
    pub asset: Option<String>,
    pub status: String,
    pub shot: String,
    pub characters: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct GalleryEntry {
    /// The scene node it comes from (the tests check the gallery is the new prologue's).
    #[cfg_attr(not(test), allow(dead_code))]
    pub node_id: String,
    pub asset: String,
    pub room_name: String,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct GalleryLibrary {
    pub entries: Vec<GalleryEntry>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoryDocument {
    #[serde(rename = "$schema", default)]
    _schema_ref: Option<String>,
    format: String,
    schema: u32,
    revision: u32,
    id: String,
    title: String,
    start: String,
    #[serde(default = "default_continue_label")]
    continue_label: String,
    #[serde(default)]
    release: ReleaseSpec,
    #[serde(default)]
    variables: BTreeMap<String, i64>,
    rooms: Vec<RoomSpec>,
    characters: Vec<CharacterSpec>,
    nodes: Vec<NodeSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseSpec {
    #[serde(default)]
    playable_until: Option<String>,
    #[serde(default)]
    locked_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomSpec {
    id: String,
    name: String,
    background: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterSpec {
    id: String,
    name: String,
    role: String,
    visual_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeSpec {
    id: String,
    room: String,
    speaker: String,
    text: String,
    visual: VisualSpec,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    next_label: Option<String>,
    #[serde(default)]
    choices: Vec<ChoiceSpec>,
    #[serde(default)]
    on_enter: Vec<EffectSpec>,
    #[serde(default)]
    on_first_enter: Vec<EffectSpec>,
    #[serde(default)]
    ending: bool,
    #[serde(default = "default_true")]
    show_hud: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    world_entry: Option<WorldEntrySpec>,
    /// A crossroads nobody sees: entering it goes on at once to the first of `branches` whose
    /// conditions hold, else to `next`. (The guiones' `@si` in visual-novel scenes.)
    #[serde(default, skip_serializing_if = "is_false")]
    pass: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    branches: Vec<BranchSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title_card: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keys: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    sound: bool,
    /// How the speaker looks saying it (a guion's `**Chiquita (enfadada):**`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mood: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// One way out of a crossroads: taken when all of `when` holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchSpec {
    #[serde(default)]
    when: Vec<ConditionSpec>,
    to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldEntrySpec {
    world: String,
    spawn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualSpec {
    #[serde(default)]
    asset: Option<String>,
    #[serde(default)]
    layers: Vec<SceneLayerSpec>,
    description: String,
    shot: String,
    #[serde(default)]
    characters: Vec<String>,
    #[serde(default)]
    status: VisualStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneLayerSpec {
    asset: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    #[serde(default = "default_layer_opacity")]
    opacity: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VisualStatus {
    Ready,
    Reference,
    Optional,
    #[default]
    Needed,
}

impl VisualStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Reference => "reference",
            Self::Optional => "optional",
            Self::Needed => "needed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceSpec {
    id: String,
    label: String,
    to: String,
    #[serde(default)]
    visible_if: Vec<ConditionSpec>,
    #[serde(default)]
    enabled_if: Vec<ConditionSpec>,
    #[serde(default)]
    disabled_reason: Option<String>,
    #[serde(default)]
    effects: Vec<EffectSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConditionSpec {
    var: String,
    op: ConditionOp,
    value: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConditionOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectSpec {
    op: EffectOp,
    var: String,
    value: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EffectOp {
    Set,
    Add,
    Subtract,
    Min,
    Max,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExplorationState {
    pub world: String,
    pub x_milli: i32,
    pub y_milli: i32,
    pub facing_x_milli: i16,
    pub facing_y_milli: i16,
    #[serde(default)]
    pub seen_interactions: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StoryState {
    story_id: String,
    story_revision: u32,
    node: String,
    variables: BTreeMap<String, i64>,
    #[serde(default)]
    visited: BTreeMap<String, u32>,
    #[serde(default)]
    history: Vec<HistoryEntry>,
    #[serde(default)]
    exploration: Option<ExplorationState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct HistoryEntry {
    node: String,
    choice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveFile {
    format: String,
    schema: u32,
    saved_at: u64,
    state: StoryState,
    #[serde(default)]
    rollback: Vec<StoryState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    world3d: Option<crate::exploration3d_state::State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thumbnail: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryFile {
    format: String,
    schema: u32,
    #[serde(default)]
    unlocked_assets: BTreeSet<String>,
}

impl Default for GalleryFile {
    fn default() -> Self {
        Self {
            format: GALLERY_FORMAT.to_string(),
            schema: GALLERY_SCHEMA,
            unlocked_assets: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedChoice {
    id: String,
    label: String,
    target: String,
    enabled: bool,
    disabled_reason: Option<String>,
    effects: Vec<EffectSpec>,
}

struct StoryRuntime {
    document: StoryDocument,
    nodes: HashMap<String, usize>,
    rooms: HashMap<String, usize>,
    characters: HashMap<String, usize>,
    warnings: Vec<String>,
}

impl StoryRuntime {
    fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("no se pudo leer la historia {}", path.display()))?;
        Self::from_bytes(&bytes, &path.display().to_string())
    }

    fn load_default() -> Result<Self> {
        if let Some(path) = story_override_path() {
            return Self::load(&path);
        }
        if let Some(bytes) = crate::portable::story_bytes() {
            return Self::from_bytes(bytes, "historia incrustada");
        }
        Self::load(&story_path())
    }

    fn from_bytes(bytes: &[u8], source: &str) -> Result<Self> {
        let document: StoryDocument =
            serde_json::from_slice(bytes).with_context(|| format!("JSON inválido en {source}"))?;
        // The guiones' visual-novel scenes join the story (if they make it invalid, it plays without
        // them and says why).
        let document = match with_guiones(&document) {
            Some(merged) => match validate_document(&merged) {
                Ok(_) => merged,
                Err(error) => {
                    eprintln!("[guion] las escenas de VN de los guiones no entran en la historia: {error:#}");
                    document
                }
            },
            None => document,
        };
        let warnings = validate_document(&document)
            .with_context(|| format!("historia inválida en {source}"))?;
        let nodes = document
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect();
        let rooms = document
            .rooms
            .iter()
            .enumerate()
            .map(|(index, room)| (room.id.clone(), index))
            .collect();
        let characters = document
            .characters
            .iter()
            .enumerate()
            .map(|(index, character)| (character.id.clone(), index))
            .collect();
        Ok(Self {
            document,
            nodes,
            rooms,
            characters,
            warnings,
        })
    }

    fn summary(&self) -> StorySummary {
        StorySummary {
            id: self.document.id.clone(),
            title: self.document.title.clone(),
            revision: self.document.revision,
            rooms: self.document.rooms.len(),
            characters: self.document.characters.len(),
            nodes: self.document.nodes.len(),
            image_briefs: self.document.nodes.len(),
            warnings: self.warnings.clone(),
        }
    }

    fn image_plan(&self) -> Vec<ImagePlanEntry> {
        self.document
            .nodes
            .iter()
            .filter(|node| !node.pass)
            .map(|node| ImagePlanEntry {
                node: node.id.clone(),
                room: node.room.clone(),
                speaker: self
                    .character(&node.speaker)
                    .map(|character| character.name.clone())
                    .unwrap_or_else(|| node.speaker.clone()),
                asset: node.visual.asset.clone(),
                status: node.visual.status.as_str().to_string(),
                shot: node.visual.shot.clone(),
                characters: node.visual.characters.clone(),
                description: node.visual.description.clone(),
            })
            .collect()
    }

    fn gallery_library(&self, unlocked_assets: &BTreeSet<String>) -> GalleryLibrary {
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        let mut total = 0;
        for node in &self.document.nodes {
            if self.is_locked_for_release(node) {
                continue;
            }
            let Some(asset) = gallery_asset(node) else {
                continue;
            };
            if !seen.insert(asset.to_string()) {
                continue;
            }
            total += 1;
            if !unlocked_assets.contains(asset) {
                continue;
            }
            let room = self.room(&node.room).expect("validated gallery room");
            let speaker = self
                .character(&node.speaker)
                .map(|character| character.name.clone())
                .unwrap_or_else(|| node.speaker.clone());
            entries.push(GalleryEntry {
                node_id: node.id.clone(),
                asset: asset.to_string(),
                room_name: room.name.clone(),
                speaker,
                text: node.text.clone(),
            });
        }
        GalleryLibrary { entries, total }
    }

    fn gallery_assets_for_state(&self, state: &StoryState) -> BTreeSet<String> {
        state
            .visited
            .keys()
            .filter_map(|id| self.node(id).ok())
            .filter(|node| !self.is_locked_for_release(node))
            .filter_map(|node| gallery_asset(node).map(str::to_owned))
            .collect()
    }

    fn localization_keys(&self) -> (Vec<String>, Vec<String>) {
        let mut nodes = Vec::new();
        let mut choices = Vec::new();
        for node in &self.document.nodes {
            if node.pass || node.tags.iter().any(|tag| tag == SIDE_VN_TAG) {
                continue;
            }
            if self.is_locked_for_release(node) {
                continue;
            }
            nodes.push(node.id.clone());
            choices.extend(
                node.choices
                    .iter()
                    .map(|choice| format!("{}.{}", node.id, choice.id)),
            );
        }
        (nodes, choices)
    }

    fn storyboard_markdown(&self) -> String {
        let mut output = String::new();
        writeln!(output, "# {} — Guion y storyboard\n", self.document.title).unwrap();
        writeln!(
            output,
            "> Generado desde `story/prologue.json` (revisión {}). Cada bloque pone la ficha visual inmediatamente después del diálogo.\n",
            self.document.revision
        )
        .unwrap();
        writeln!(
            output,
            "**Resumen:** {} cuartos · {} personajes · {} diálogos/fichas de imagen.\n",
            self.document.rooms.len(),
            self.document.characters.len(),
            self.document.nodes.len()
        )
        .unwrap();

        for (index, node) in self.document.nodes.iter().enumerate() {
            if node.pass {
                continue;
            }
            let room = self.room(&node.room).expect("validated room");
            let speaker = self
                .character(&node.speaker)
                .map(|character| character.name.as_str())
                .unwrap_or(node.speaker.as_str());
            writeln!(output, "## {:02} · `{}`\n", index + 1, node.id).unwrap();
            writeln!(output, "**Cuarto:** {} (`{}`)\n", room.name, room.id).unwrap();
            writeln!(output, "**{}:** {}\n", speaker, node.text).unwrap();
            writeln!(output, "> **Ficha de imagen**").unwrap();
            writeln!(output, "> - **Descripción:** {}", node.visual.description).unwrap();
            writeln!(output, "> - **Encuadre:** {}", node.visual.shot).unwrap();
            let characters = if node.visual.characters.is_empty() {
                "sin personajes en cámara".to_string()
            } else {
                node.visual
                    .characters
                    .iter()
                    .map(|id| {
                        self.character(id)
                            .map(|character| character.name.clone())
                            .unwrap_or_else(|| id.clone())
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            writeln!(output, "> - **Personajes:** {characters}").unwrap();
            writeln!(output, "> - **Estado:** `{}`", node.visual.status.as_str()).unwrap();
            writeln!(
                output,
                "> - **Asset:** `{}`\n",
                node.visual.asset.as_deref().unwrap_or("pendiente")
            )
            .unwrap();

            if let Some(next) = &node.next {
                writeln!(output, "**Continúa en:** `{next}`\n").unwrap();
            } else if !node.choices.is_empty() {
                writeln!(output, "**Selecciones:**\n").unwrap();
                for choice in &node.choices {
                    writeln!(
                        output,
                        "- **{}** (`{}`) → `{}`",
                        choice.label, choice.id, choice.to
                    )
                    .unwrap();
                }
                output.push('\n');
            } else if node.ending {
                output.push_str("**Final del bloque.**\n\n");
            }
            output.push_str("---\n\n");
        }
        output
    }

    fn new_state(&self) -> Result<StoryState> {
        let mut state = StoryState {
            story_id: self.document.id.clone(),
            story_revision: self.document.revision,
            node: String::new(),
            variables: self.document.variables.clone(),
            visited: BTreeMap::new(),
            history: Vec::new(),
            exploration: None,
        };
        self.enter_node(&mut state, &self.document.start)?;
        Ok(state)
    }

    fn node(&self, id: &str) -> Result<&NodeSpec> {
        self.nodes
            .get(id)
            .map(|index| &self.document.nodes[*index])
            .ok_or_else(|| anyhow!("nodo inexistente: {id}"))
    }

    fn is_release_ending(&self, node: &NodeSpec) -> bool {
        self.document.release.playable_until.as_deref() == Some(node.id.as_str())
    }

    fn is_locked_for_release(&self, node: &NodeSpec) -> bool {
        node.tags.iter().any(|tag| {
            self.document
                .release
                .locked_tags
                .iter()
                .any(|locked| locked == tag)
        })
    }

    fn effective_ending(&self, node: &NodeSpec) -> bool {
        node.ending || self.is_release_ending(node)
    }

    fn release_safe_target(&self, requested: &str) -> Result<String> {
        let node = self.node(requested)?;
        if !self.is_locked_for_release(node) {
            return Ok(requested.to_string());
        }
        self.document
            .release
            .playable_until
            .clone()
            .ok_or_else(|| anyhow!("el nodo {requested} está bloqueado en esta versión"))
    }

    fn room(&self, id: &str) -> Result<&RoomSpec> {
        self.rooms
            .get(id)
            .map(|index| &self.document.rooms[*index])
            .ok_or_else(|| anyhow!("cuarto inexistente: {id}"))
    }

    fn character(&self, id: &str) -> Option<&CharacterSpec> {
        self.characters
            .get(id)
            .map(|index| &self.document.characters[*index])
    }

    fn view(&self, state: &StoryState) -> Result<SceneView> {
        let node = self.node(&state.node)?;
        let room = self.room(&node.room)?;
        let speaker = self
            .character(&node.speaker)
            .map(|character| character.name.clone())
            .unwrap_or_else(|| node.speaker.clone());
        let ending = self.effective_ending(node);
        let choices = if ending {
            Vec::new()
        } else {
            self.resolved_choices(state, node)
                .into_iter()
                .map(|choice| Choice {
                    id: choice.id,
                    label: choice.label,
                    enabled: choice.enabled,
                    disabled_reason: choice.disabled_reason,
                })
                .collect()
        };
        Ok(SceneView {
            id: node.id.clone(),
            room_id: node.room.clone(),
            room_name: room.name.clone(),
            bg: room.background.clone(),
            speaker_id: node.speaker.clone(),
            speaker,
            text: node.text.clone(),
            image_asset: node.visual.asset.clone().filter(|asset| !asset.is_empty()),
            layers: node
                .visual
                .layers
                .iter()
                .map(|layer| SceneLayer {
                    asset: layer.asset.clone(),
                    x: layer.x,
                    y: layer.y,
                    width: layer.width,
                    height: layer.height,
                    opacity: layer.opacity.clamp(0.0, 1.0),
                })
                .collect(),
            choices,
            ending,
            show_hud: node.show_hud,
            world_entry: node.world_entry.as_ref().map(|entry| WorldEntry {
                world: entry.world.clone(),
                spawn: entry.spawn.clone(),
            }),
            title_card: node.title_card.clone(),
            keys: node.keys.clone(),
            sound: node.sound,
            mood: node.mood.clone(),
        })
    }

    fn decide(&self, state: &mut StoryState, index: usize) -> Result<()> {
        let current = self.node(&state.node)?.clone();
        if self.effective_ending(&current) {
            bail!("el nodo {} ya es un final", current.id);
        }
        let choices = self.resolved_choices(state, &current);
        let choice = choices
            .get(index)
            .ok_or_else(|| anyhow!("decisión {index} fuera de rango en {}", current.id))?;
        if !choice.enabled {
            bail!(
                "decisión {} deshabilitada: {}",
                choice.id,
                choice
                    .disabled_reason
                    .as_deref()
                    .unwrap_or("condición no cumplida")
            );
        }
        apply_effects(&mut state.variables, &choice.effects);
        state.history.push(HistoryEntry {
            node: current.id,
            choice: choice.id.clone(),
        });
        self.enter_node(state, &choice.target)
    }

    fn resolved_choices(&self, state: &StoryState, node: &NodeSpec) -> Vec<ResolvedChoice> {
        if !node.choices.is_empty() {
            return node
                .choices
                .iter()
                .filter(|choice| conditions_match(&state.variables, &choice.visible_if))
                .map(|choice| ResolvedChoice {
                    id: choice.id.clone(),
                    label: choice.label.clone(),
                    target: choice.to.clone(),
                    enabled: conditions_match(&state.variables, &choice.enabled_if),
                    disabled_reason: choice.disabled_reason.clone(),
                    effects: choice.effects.clone(),
                })
                .collect();
        }
        node.next
            .as_ref()
            .map(|target| {
                vec![ResolvedChoice {
                    id: "continue".to_string(),
                    label: node
                        .next_label
                        .clone()
                        .unwrap_or_else(|| self.document.continue_label.clone()),
                    target: target.clone(),
                    enabled: true,
                    disabled_reason: None,
                    effects: Vec::new(),
                }]
            })
            .unwrap_or_default()
    }

    fn enter_node(&self, state: &mut StoryState, target: &str) -> Result<()> {
        let mut requested = target.to_string();
        // A crossroads (`pass`) is never shown: it leads on at once. A chain of them is bounded
        // so a badly written loop fails instead of hanging.
        for _ in 0..64 {
            let target = self.release_safe_target(&requested)?;
            let node = self.node(&target)?;
            let first_visit = state.visited.get(&target).copied().unwrap_or(0) == 0;
            state.node = target.clone();
            *state.visited.entry(target).or_default() += 1;
            if first_visit {
                apply_effects(&mut state.variables, &node.on_first_enter);
            }
            apply_effects(&mut state.variables, &node.on_enter);
            if !node.pass {
                return Ok(());
            }
            requested = node
                .branches
                .iter()
                .find(|branch| conditions_match(&state.variables, &branch.when))
                .map(|branch| branch.to.clone())
                .or_else(|| node.next.clone())
                .ok_or_else(|| anyhow!("el cruce {} no tiene salida", node.id))?;
        }
        bail!("demasiados cruces seguidos desde {target}: ¿un bucle entre nodos de paso?")
    }

    fn migrate_state(&self, mut state: StoryState) -> Result<StoryState> {
        if state.story_id != self.document.id {
            bail!(
                "el guardado pertenece a {}, no a {}",
                state.story_id,
                self.document.id
            );
        }
        self.node(&state.node)
            .with_context(|| format!("el guardado apunta a {}", state.node))?;
        state.node = self.release_safe_target(&state.node)?;
        // A save in the middle of the old prologue starts the new one (that lore is gone); one at
        // its end, the way into the world, still goes in there.
        let old_prologue = self
            .node(&state.node)?
            .tags
            .iter()
            .any(|tag| tag == OLD_PROLOGUE_TAG);
        if old_prologue
            && self.document.release.playable_until.as_deref() != Some(state.node.as_str())
        {
            state.node = self.document.start.clone();
        }
        for (key, value) in &self.document.variables {
            state.variables.entry(key.clone()).or_insert(*value);
        }
        state.story_revision = self.document.revision;
        Ok(state)
    }
}

pub struct Game {
    story: StoryRuntime,
    state: StoryState,
    /// The states to step back to. Shared (`Arc`): the autosave, every second in the world, takes
    /// a copy of the list for the save writer's thread, which costs nothing this way.
    rollback: Vec<Arc<StoryState>>,
    /// Separate history for a VN launched from the 3D world.  It must not
    /// consume the prologue history, but the wheel still needs to step back
    /// through the dialogue before closing the side scene.
    world_vn_rollback: Vec<Arc<StoryState>>,
    world_vn_active: bool,
    autosave: bool,
    last_autosave_error: Option<String>,
    world3d: Option<crate::exploration3d_state::State>,
}

impl Game {
    pub fn load() -> Result<Self> {
        let story = StoryRuntime::load_default()?;
        Self::from_story(story)
    }

    fn from_story(story: StoryRuntime) -> Result<Self> {
        let state = story.new_state()?;
        Ok(Self {
            story,
            state,
            rollback: Vec::new(),
            world_vn_rollback: Vec::new(),
            world_vn_active: false,
            autosave: autosave_allowed(),
            last_autosave_error: None,
            world3d: None,
        })
    }

    pub fn new_game(&mut self) -> Result<()> {
        self.state = self.story.new_state()?;
        self.rollback.clear();
        self.world_vn_rollback.clear();
        self.world_vn_active = false;
        self.world3d = None;
        if self.autosave {
            // A new VN playthrough must not inherit an older run's map phase.
            let path = crate::exploration3d_state::path();
            if let Err(error) = fs::remove_file(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(error)
                        .with_context(|| format!("reset 3D checkpoint {}", path.display()));
                }
            }
        }
        self.autosave_best_effort();
        Ok(())
    }

    pub fn continue_game(&mut self) -> Result<()> {
        self.load_from_path(&continue_path())?;
        self.merge_3d_checkpoint();
        // Persist the merged transform so returning to the VN and launching
        // the 3D runtime again uses one canonical checkpoint.
        self.autosave_best_effort();
        self.record_gallery_best_effort();
        Ok(())
    }

    fn merge_3d_checkpoint(&mut self) {
        if !self.autosave || self.state.exploration.is_none() {
            return;
        }
        let Ok(Some(session)) = crate::exploration3d_state::load() else {
            return;
        };
        // New saves contain their own complete snapshot; only a newer sidecar may supersede it.
        if self
            .world3d
            .as_ref()
            .is_some_and(|saved| saved.saved_at >= session.saved_at)
        {
            return;
        }
        let expected_world = self
            .story
            .view(&self.state)
            .ok()
            .and_then(|view| view.world_entry.map(|entry| entry.world));
        if expected_world.as_deref() != Some(session.world.as_str()) {
            return;
        }
        let milli = |value: f32| -> i32 {
            if value.is_finite() {
                (value * 1000.0)
                    .round()
                    .clamp(i32::MIN as f32, i32::MAX as f32) as i32
            } else {
                0
            }
        };
        let facing = |value: f32| -> i16 {
            if value.is_finite() {
                (value * 1000.0)
                    .round()
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16
            } else {
                0
            }
        };
        self.state.exploration = Some(ExplorationState {
            world: session.world.clone(),
            x_milli: milli(session.x),
            y_milli: milli(session.z),
            facing_x_milli: facing(session.yaw.sin()),
            facing_y_milli: facing(session.yaw.cos()),
            seen_interactions: session.seen_interactions.clone(),
        });
        self.world3d = Some(session);
    }

    pub fn view(&mut self) -> Result<SceneView> {
        self.story.view(&self.state)
    }

    pub fn stats(&mut self) -> Result<Stats> {
        let get = |key: &str| self.state.variables.get(key).copied().unwrap_or(0);
        Ok(Stats {
            fear: get("fear"),
            devotion: get("devotion"),
            dignity: get("dignity"),
            hunger: get("hunger"),
            decree: get("decree"),
            desire: get("deseo"),
            suspicion: get("suspicion"),
            maids_trust: get("maids_trust"),
        })
    }

    pub fn gallery_library(&self) -> Result<GalleryLibrary> {
        let mut unlocked = load_gallery_file(&gallery_path())?.unlocked_assets;
        unlocked.extend(self.story.gallery_assets_for_state(&self.state));
        Ok(self.story.gallery_library(&unlocked))
    }

    pub fn decide(&mut self, index: usize) -> Result<()> {
        let previous = self.state.clone();
        self.story.decide(&mut self.state, index)?;
        let rollback = if self.world_vn_active {
            &mut self.world_vn_rollback
        } else {
            &mut self.rollback
        };
        rollback.push(Arc::new(previous));
        if rollback.len() > ROLLBACK_LIMIT {
            rollback.remove(0);
        }
        self.autosave_best_effort();
        Ok(())
    }

    /// Enter a world-launched VN block without resetting the current map
    /// checkpoint.  The exploration host calls this for interactions such as
    /// the mage; returning through a node with `world_entry` restores the same
    /// 3D session in the existing window.
    pub fn jump_to_node(&mut self, node: &str) -> Result<()> {
        self.story.enter_node(&mut self.state, node)?;
        // A world-launched side VN is a separate interaction, not a normal
        // prologue branch.  Keeping the prologue snapshot in the shared
        // rollback stack made WHEEL/PageUp jump straight back to the intro
        // instead of returning to the 3D world.
        self.rollback.clear();
        self.world_vn_rollback.clear();
        self.world_vn_active = true;
        self.autosave_best_effort();
        Ok(())
    }

    pub fn rollback_world_vn(&mut self) -> Result<bool> {
        if !self.world_vn_active {
            return Ok(false);
        }
        let Some(previous) = self.world_vn_rollback.pop() else {
            return Ok(false);
        };
        self.state = self.story.migrate_state(Arc::unwrap_or_clone(previous))?;
        self.autosave_best_effort();
        Ok(true)
    }

    pub fn finish_world_vn(&mut self) {
        self.world_vn_rollback.clear();
        self.world_vn_active = false;
    }

    pub fn rollback(&mut self) -> Result<bool> {
        let Some(previous) = self.rollback.pop() else {
            return Ok(false);
        };
        self.state = self.story.migrate_state(Arc::unwrap_or_clone(previous))?;
        self.autosave_best_effort();
        Ok(true)
    }

    pub fn can_rollback(&self) -> bool {
        !self.rollback.is_empty()
    }

    /// Does this game write its continue save as it goes?
    pub fn autosaves(&self) -> bool {
        self.autosave
    }

    pub fn take_autosave_error(&mut self) -> Option<String> {
        self.last_autosave_error.take()
    }

    pub fn save_continue(&self) -> Result<()> {
        self.save_to_path(&continue_path())
    }

    /// The continue save, made into JSON and written by the save writer: here only shared copies
    /// are taken (the history is megabytes; turning it into text took 25-40 ms every second in
    /// the world). Nothing is written when only its time changed.
    fn save_continue_in_background(&self) -> Result<()> {
        use std::hash::{Hash, Hasher};
        let mut world3d = self.world3d.clone();
        let saved_at = now_unix();
        // The key: the state, the place (without the time of saving) and which history it is.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_vec(&self.state)?.hash(&mut hasher);
        if let Some(world) = world3d.as_mut() {
            let at = world.saved_at;
            world.saved_at = 0;
            serde_json::to_vec(world)?.hash(&mut hasher);
            world.saved_at = at;
        }
        for state in &self.rollback {
            (Arc::as_ptr(state) as usize).hash(&mut hasher);
        }
        let key = hasher.finish();
        let state = self.state.clone();
        let rollback = self.rollback.clone();
        crate::exploration3d_state::background::write_with(
            continue_path(),
            true,
            key,
            Box::new(move || {
                #[derive(Serialize)]
                struct Out<'a> {
                    format: &'a str,
                    schema: u32,
                    saved_at: u64,
                    state: &'a StoryState,
                    rollback: Vec<&'a StoryState>,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    world3d: Option<&'a crate::exploration3d_state::State>,
                }
                let out = Out {
                    format: SAVE_FORMAT,
                    schema: SAVE_SCHEMA,
                    saved_at,
                    state: &state,
                    rollback: rollback.iter().map(|s| &**s).collect(),
                    world3d: world3d.as_ref(),
                };
                serde_json::to_vec_pretty(&out).unwrap_or_default()
            }),
        );
        Ok(())
    }

    /// The story's variables (shared with the guiones in the 3D world).
    pub fn story_variables(&self) -> &BTreeMap<String, i64> {
        &self.state.variables
    }

    /// Takes the 3D world's values for the variables this story declares.
    pub fn set_story_variables(&mut self, values: &BTreeMap<String, i32>) {
        for (key, value) in values {
            if let Some(slot) = self.state.variables.get_mut(key) {
                *slot = i64::from(*value);
            } else if WORLD_ONLY_VARIABLES.contains(&key.as_str()) || key.starts_with("ero_") {
                // Born in the 3D world (the bed, the brothel...): the VN reads them too.
                self.state.variables.insert(key.clone(), i64::from(*value));
            }
        }
    }

    pub fn exploration_state(&self) -> Option<&ExplorationState> {
        self.state.exploration.as_ref()
    }

    pub fn world_snapshot(&self) -> Option<&crate::exploration3d_state::State> {
        self.world3d.as_ref()
    }

    /// The bag as she has just laid it out in the pause menu (saved with the next save).
    pub fn set_world_backpack(&mut self, pack: crate::exploration3d::backpack::Backpack) {
        if let Some(world) = self.world3d.as_mut() {
            world.backpack = Some(pack);
        }
    }

    pub fn checkpoint_3d(&mut self, state: crate::exploration3d_state::State) {
        self.state.exploration = Some(ExplorationState {
            world: state.world.clone(),
            x_milli: (state.x * 1000.0).round() as i32,
            y_milli: (state.z * 1000.0).round() as i32,
            facing_x_milli: (state.yaw.sin() * 1000.0).round() as i16,
            facing_y_milli: (state.yaw.cos() * 1000.0).round() as i16,
            seen_interactions: state.seen_interactions.clone(),
        });
        self.world3d = Some(state);
        self.autosave_best_effort();
    }

    /// Read-only history: opening the log must never roll back or unlock scenes.
    pub fn history_views(&self) -> Vec<SceneView> {
        self.rollback
            .iter()
            .map(|state| &**state)
            .chain(std::iter::once(&self.state))
            .filter_map(|state| self.story.view(state).ok())
            .collect()
    }

    pub fn save_slot(&self, index: usize, thumbnail: Option<Vec<u8>>) -> Result<()> {
        let path = slot_path(index)?;
        self.save_manual_to_path(&path, thumbnail)
    }

    fn save_manual_to_path(&self, path: &Path, thumbnail: Option<Vec<u8>>) -> Result<()> {
        let mut file = self.save_file();
        file.thumbnail = thumbnail;
        if path.is_file() {
            fs::copy(&path, path.with_extension("json.bak"))
                .context("no se pudo conservar el guardado anterior")?;
        }
        write_json_atomic(path, &file)
    }

    /// The auto slot is read-only; loading it uses the same validation as manual saves.
    pub fn load_slot(&mut self, index: usize) -> Result<()> {
        let path = if index == MANUAL_SLOTS {
            continue_path()
        } else {
            slot_path(index)?
        };
        self.load_from_path(&path)?;
        if index == MANUAL_SLOTS {
            self.merge_3d_checkpoint();
        }
        Ok(())
    }

    pub fn save_slots(&self) -> Vec<Option<SaveSlot>> {
        (0..=MANUAL_SLOTS)
            .map(|index| {
                let path = if index == MANUAL_SLOTS {
                    continue_path()
                } else {
                    slot_path(index).ok()?
                };
                if !path.exists() {
                    return None;
                }
                let result = (|| -> Result<SaveSlot> {
                    let file: SaveFile = serde_json::from_slice(&fs::read(&path)?)?;
                    self.validate_save(&file)?;
                    let migrated = self.story.migrate_state(file.state)?;
                    Ok(SaveSlot {
                        saved_at: file.saved_at,
                        is_world: migrated.exploration.is_some(),
                        scene: Some(self.story.view(&migrated)?),
                        thumbnail: file.thumbnail,
                        error: None,
                    })
                })();
                Some(result.unwrap_or_else(|error| SaveSlot {
                    saved_at: 0,
                    scene: None,
                    is_world: false,
                    thumbnail: None,
                    error: Some(error.to_string()),
                }))
            })
            .collect()
    }

    fn autosave_best_effort(&mut self) {
        if !self.autosave {
            self.last_autosave_error = None;
            return;
        }
        // On the save writer's thread: this runs every second while she walks in the world.
        let save_error = self.save_continue_in_background().err();
        let gallery_error = self.record_gallery().err();
        self.last_autosave_error = save_error
            .or(gallery_error)
            .map(|error| format!("{error:#}"));
    }

    fn record_gallery_best_effort(&mut self) {
        if !self.autosave {
            return;
        }
        if let Err(error) = self.record_gallery() {
            self.last_autosave_error = Some(format!("{error:#}"));
        }
    }

    fn record_gallery(&self) -> Result<()> {
        // Asked every second in the world: read and written only when something new shows.
        static KNOWN: std::sync::Mutex<Option<(PathBuf, BTreeSet<String>)>> =
            std::sync::Mutex::new(None);
        let path = gallery_path();
        let wanted = self.story.gallery_assets_for_state(&self.state);
        let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((known_path, assets)) = known.as_ref() {
            if *known_path == path && wanted.iter().all(|a| assets.contains(a)) {
                return Ok(());
            }
        }
        // The file as it is on disk (a write of it may still be on its way).
        crate::exploration3d_state::background::flush();
        let mut file = load_gallery_file(&path)?;
        file.unlocked_assets.extend(wanted);
        let bytes = serde_json::to_vec_pretty(&file).context("no se pudo serializar la galería")?;
        let key = crate::exploration3d_state::background::key_of(&bytes);
        crate::exploration3d_state::background::write(path.clone(), bytes, true, key);
        *known = Some((path, file.unlocked_assets.iter().cloned().collect()));
        Ok(())
    }

    fn save_to_path(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, &self.save_file())
    }

    fn save_file(&self) -> SaveFile {
        SaveFile {
            format: SAVE_FORMAT.to_string(),
            schema: SAVE_SCHEMA,
            saved_at: now_unix(),
            state: self.state.clone(),
            rollback: self
                .rollback
                .iter()
                .map(|state| (**state).clone())
                .collect(),
            world3d: self.world3d.clone(),
            thumbnail: None,
        }
    }

    fn validate_save(&self, file: &SaveFile) -> Result<()> {
        if file.format != SAVE_FORMAT || file.schema != SAVE_SCHEMA {
            bail!("guardado incompatible");
        }
        self.story.migrate_state(file.state.clone())?;
        if let Some(world) = &file.world3d {
            anyhow::ensure!(
                world.format == crate::exploration3d_state::FORMAT
                    && world.schema == crate::exploration3d_state::SCHEMA
                    && world.x.is_finite()
                    && world.z.is_finite()
                    && world.yaw.is_finite(),
                "estado 3D inválido"
            );
            anyhow::ensure!(
                file.state
                    .exploration
                    .as_ref()
                    .is_some_and(|e| e.world == world.world),
                "el mapa no corresponde al guardado"
            );
        }
        Ok(())
    }

    fn load_from_path(&mut self, path: &Path) -> Result<()> {
        let bytes = fs::read(path)
            .with_context(|| format!("no se pudo leer el guardado {}", path.display()))?;
        let file: SaveFile = serde_json::from_slice(&bytes)
            .with_context(|| format!("guardado corrupto en {}", path.display()))?;
        self.validate_save(&file)?;
        if file.format != SAVE_FORMAT || file.schema != SAVE_SCHEMA {
            bail!(
                "guardado incompatible: formato={} schema={}",
                file.format,
                file.schema
            );
        }
        self.state = self.story.migrate_state(file.state)?;
        self.rollback = file
            .rollback
            .into_iter()
            .filter_map(|snapshot| self.story.migrate_state(snapshot).ok())
            .rev()
            .take(ROLLBACK_LIMIT)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(Arc::new)
            .collect();
        self.last_autosave_error = None;
        self.world3d = file.world3d;
        Ok(())
    }
}

fn slot_path(index: usize) -> Result<PathBuf> {
    anyhow::ensure!(index < MANUAL_SLOTS, "ranura inválida");
    Ok(continue_path()
        .parent()
        .context("carpeta de guardados")?
        .join("slots")
        .join(format!("slot_{:02}.json", index + 1)))
}

pub fn validate_story_path(path: &Path) -> Result<StorySummary> {
    StoryRuntime::load(path).map(|story| story.summary())
}

pub fn validate_default_story() -> Result<StorySummary> {
    StoryRuntime::load_default().map(|story| story.summary())
}

pub fn image_plan(path: &Path) -> Result<Vec<ImagePlanEntry>> {
    StoryRuntime::load(path).map(|story| story.image_plan())
}

pub fn default_image_plan() -> Result<Vec<ImagePlanEntry>> {
    StoryRuntime::load_default().map(|story| story.image_plan())
}

pub fn storyboard_markdown(path: &Path) -> Result<String> {
    StoryRuntime::load(path).map(|story| story.storyboard_markdown())
}

pub fn default_storyboard_markdown() -> Result<String> {
    StoryRuntime::load_default().map(|story| story.storyboard_markdown())
}

pub fn gallery_library() -> Result<GalleryLibrary> {
    let story = StoryRuntime::load_default()?;
    let unlocked = load_gallery_file(&gallery_path())?.unlocked_assets;
    Ok(story.gallery_library(&unlocked))
}

pub fn localization_keys(path: &Path) -> Result<(Vec<String>, Vec<String>)> {
    StoryRuntime::load(path).map(|story| story.localization_keys())
}

pub fn default_localization_keys() -> Result<(Vec<String>, Vec<String>)> {
    StoryRuntime::load_default().map(|story| story.localization_keys())
}

pub fn has_continue() -> bool {
    let Ok(story) = StoryRuntime::load_default() else {
        return false;
    };
    let Ok(bytes) = fs::read(continue_path()) else {
        return false;
    };
    let Ok(file) = serde_json::from_slice::<SaveFile>(&bytes) else {
        return false;
    };
    file.format == SAVE_FORMAT
        && file.schema == SAVE_SCHEMA
        && story.migrate_state(file.state).is_ok()
}

pub fn story_path() -> PathBuf {
    if let Some(path) = story_override_path() {
        return path;
    }
    [
        PathBuf::from("demos/demon-queen/story/prologue.json"),
        PathBuf::from("story/prologue.json"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| crate::exploration3d_state::source_root().join("story/prologue.json"))
}

fn story_override_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(position) = args.iter().position(|argument| argument == "--story") {
        if let Some(path) = args.get(position + 1) {
            return Some(PathBuf::from(path));
        }
    }
    std::env::var_os("DEMON_QUEEN_STORY").map(PathBuf::from)
}

pub fn continue_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SAVES") {
        return PathBuf::from(path).join("continue.json");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("saves")
            .join("continue.json");
    }
    crate::exploration3d_state::source_root().join("data/saves/continue.json")
}

pub fn gallery_path() -> PathBuf {
    crate::exploration3d::gallery_seen::gallery_path()
}

/// Everything the gallery has been shown: the pictures' keys and the scenes' shots
/// (`gallery_seen::PREFIX`).
pub fn gallery_unlocked() -> BTreeSet<String> {
    load_gallery_file(&gallery_path())
        .map(|file| file.unlocked_assets)
        .unwrap_or_default()
}

fn load_gallery_file(path: &Path) -> Result<GalleryFile> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(GalleryFile::default());
    };
    let file: GalleryFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("galería corrupta en {}", path.display()))?;
    if file.format != GALLERY_FORMAT || file.schema != GALLERY_SCHEMA {
        bail!("galería incompatible en {}", path.display());
    }
    Ok(file)
}

fn default_continue_label() -> String {
    "Continuar".to_string()
}

fn default_true() -> bool {
    true
}

fn default_layer_opacity() -> f32 {
    1.0
}

fn autosave_allowed() -> bool {
    !cfg!(test)
        && !std::env::args().any(|argument| {
            argument.starts_with("--capture")
                || matches!(
                    argument.as_str(),
                    "--headless" | "--validate-story" | "--image-plan" | "--storyboard-md"
                )
        })
}

/// The new prologue's first scene (`data/guiones/prologo.md`, made by `tools/prologo_vn.py`).
const NEW_PROLOGUE: &str = "prologo.p01_inicio";
/// The start of the new prologue's node ids (every scene of the guion `prologo`).
pub const PROLOGUE_NODE_PREFIX: &str = "g_prologo__";
/// Where the guion visual novels go on into the world (the prologue ends there): world, spawn.
pub const GUION_WORLD_ENTRY: (&str, &str) = ("act1_castle", "bedroom_inside");

/// The story with the guiones' `vn` scenes added (their nodes, the characters and variables they
/// use), or `None` when there are none.
/// The picture a node adds to the gallery: its CG when it is ready, or a guion scene's picture
/// (its last layer, covering the whole canvas: a `@cg`). The old prologue's are out of it.
fn gallery_asset(node: &NodeSpec) -> Option<&str> {
    if node.tags.iter().any(|tag| tag == OLD_PROLOGUE_TAG) {
        return None;
    }
    if node.visual.status == VisualStatus::Ready {
        if let Some(asset) = node
            .visual
            .asset
            .as_deref()
            .filter(|asset| !asset.is_empty())
        {
            return Some(asset);
        }
    }
    node.visual
        .layers
        .last()
        .filter(|l| l.x == 0 && l.y == 0 && l.width >= 1920 && l.height >= 1080)
        .filter(|l| l.asset.starts_with("native_1080p/"))
        .map(|l| l.asset.as_str())
}

fn with_guiones(document: &StoryDocument) -> Option<StoryDocument> {
    use crate::exploration3d::guion;
    let lib = guion::library();
    let room = document
        .rooms
        .iter()
        .find(|r| r.id == "royal_bedroom")
        .or(document.rooms.first())?
        .id
        .clone();
    let world = serde_json::json!({ "world": GUION_WORLD_ENTRY.0, "spawn": GUION_WORLD_ENTRY.1 });
    let layout = guion::VnLayout {
        tags: vec![SIDE_VN_TAG.into(), "guion".into()],
        ..Default::default()
    };
    let guion::VnStory {
        nodes,
        characters,
        variables,
    } = guion::compile_vn(lib, &room, world, &layout);
    if nodes.is_empty() {
        return None;
    }
    let mut merged = document.clone();
    for (id, name) in characters {
        if !merged.characters.iter().any(|c| c.id == id) {
            let spec = serde_json::json!({ "id": id, "name": name, "role": "Personaje de guion", "visual_description": "Definido en los guiones." });
            merged.characters.push(serde_json::from_value(spec).ok()?);
        }
    }
    for var in variables {
        merged.variables.entry(var).or_insert(0);
    }
    for node in nodes {
        match serde_json::from_value::<NodeSpec>(node) {
            Ok(node) => merged.nodes.push(node),
            Err(error) => {
                eprintln!("[guion] nodo de VN inválido: {error}");
                return None;
            }
        }
    }
    // The prologue is the guion's («El Relevo», data/guiones/prologo.md) when it is there: a new
    // game starts at its first scene. The old one stays in the story, unused, as the fallback.
    let prologue = guion::vn_start_node(NEW_PROLOGUE);
    if merged.nodes.iter().any(|n| n.id == prologue) {
        merged.start = prologue;
        // Nothing leads to the old prologue from the new start any more: tagged as such.
        let reachable: HashSet<String> = {
            let ids: HashSet<&str> = merged.nodes.iter().map(|n| n.id.as_str()).collect();
            reachable_nodes(&merged, &ids)
                .into_iter()
                .map(str::to_owned)
                .collect()
        };
        for node in &mut merged.nodes {
            if !reachable.contains(&node.id) && !node.tags.iter().any(|tag| tag == SIDE_VN_TAG) {
                node.tags.push(OLD_PROLOGUE_TAG.to_owned());
            }
        }
    }
    Some(merged)
}

fn validate_document(document: &StoryDocument) -> Result<Vec<String>> {
    if document.format != STORY_FORMAT {
        bail!("format debe ser {STORY_FORMAT:?}");
    }
    if document.schema != STORY_SCHEMA {
        bail!(
            "schema {} no compatible; este runtime acepta {}",
            document.schema,
            STORY_SCHEMA
        );
    }
    require_id("story.id", &document.id)?;
    require_text("story.title", &document.title)?;
    require_text("story.start", &document.start)?;
    require_text("story.continue_label", &document.continue_label)?;
    if document.rooms.is_empty() {
        bail!("rooms no puede estar vacío");
    }
    if document.characters.is_empty() {
        bail!("characters no puede estar vacío");
    }
    if document.nodes.is_empty() {
        bail!("nodes no puede estar vacío");
    }

    let room_ids = unique_ids("room", document.rooms.iter().map(|room| room.id.as_str()))?;
    let character_ids = unique_ids(
        "character",
        document
            .characters
            .iter()
            .map(|character| character.id.as_str()),
    )?;
    let node_ids = unique_ids("node", document.nodes.iter().map(|node| node.id.as_str()))?;
    if !node_ids.contains(document.start.as_str()) {
        bail!("start apunta a nodo inexistente: {}", document.start);
    }
    if let Some(playable_until) = document.release.playable_until.as_deref() {
        if !node_ids.contains(playable_until) {
            bail!("release.playable_until apunta a nodo inexistente: {playable_until}");
        }
        let ending_node = document
            .nodes
            .iter()
            .find(|node| node.id == playable_until)
            .expect("playable_until already validated");
        if ending_node.tags.iter().any(|tag| {
            document
                .release
                .locked_tags
                .iter()
                .any(|locked| locked == tag)
        }) {
            bail!("release.playable_until no puede estar bloqueado: {playable_until}");
        }
    }
    for tag in &document.release.locked_tags {
        require_text("release.locked_tags", tag)?;
    }
    if let Some(start) = document.nodes.iter().find(|node| node.id == document.start) {
        if start.tags.iter().any(|tag| {
            document
                .release
                .locked_tags
                .iter()
                .any(|locked| locked == tag)
        }) {
            bail!("start no puede pertenecer a un bloque cerrado de release");
        }
    }

    for room in &document.rooms {
        require_text(&format!("room {}.name", room.id), &room.name)?;
        require_text(&format!("room {}.background", room.id), &room.background)?;
        require_text(&format!("room {}.description", room.id), &room.description)?;
    }
    for character in &document.characters {
        require_text(&format!("character {}.name", character.id), &character.name)?;
        require_text(&format!("character {}.role", character.id), &character.role)?;
        require_text(
            &format!("character {}.visual_description", character.id),
            &character.visual_description,
        )?;
    }

    for node in &document.nodes {
        require_text(&format!("node {}.text", node.id), &node.text)?;
        if !room_ids.contains(node.room.as_str()) {
            bail!("node {} usa cuarto inexistente: {}", node.id, node.room);
        }
        if !character_ids.contains(node.speaker.as_str()) {
            bail!(
                "node {} usa personaje inexistente: {}",
                node.id,
                node.speaker
            );
        }
        require_text(
            &format!("node {}.visual.description", node.id),
            &node.visual.description,
        )?;
        require_text(&format!("node {}.visual.shot", node.id), &node.visual.shot)?;
        for character in &node.visual.characters {
            if !character_ids.contains(character.as_str()) {
                bail!(
                    "node {}.visual usa personaje inexistente: {}",
                    node.id,
                    character
                );
            }
        }
        if node.choices.len() > MAX_CHOICES {
            bail!(
                "node {} tiene {} elecciones; máximo {}",
                node.id,
                node.choices.len(),
                MAX_CHOICES
            );
        }
        if node.next.is_some() && !node.choices.is_empty() {
            bail!("node {} no puede tener next y choices a la vez", node.id);
        }
        if node.ending && (node.next.is_some() || !node.choices.is_empty()) {
            bail!("node final {} no puede tener transiciones", node.id);
        }
        if let Some(entry) = &node.world_entry {
            require_id(&format!("node {}.world_entry.world", node.id), &entry.world)?;
            require_id(&format!("node {}.world_entry.spawn", node.id), &entry.spawn)?;
        }
        if !node.ending && node.next.is_none() && node.choices.is_empty() {
            bail!("node {} queda sin next, choices ni ending", node.id);
        }
        if let Some(target) = &node.next {
            if !node_ids.contains(target.as_str()) {
                bail!("node {}.next apunta a {target} inexistente", node.id);
            }
        }
        if node.pass {
            if node.next.is_none() {
                bail!(
                    "el cruce {} necesita next (por si no se cumple ninguna rama)",
                    node.id
                );
            }
            if !node.choices.is_empty() || node.ending || node.world_entry.is_some() {
                bail!(
                    "el cruce {} no puede tener decisiones, final ni salida al mundo",
                    node.id
                );
            }
        } else if !node.branches.is_empty() {
            bail!("node {} tiene branches pero no es un cruce (pass)", node.id);
        }
        for (index, branch) in node.branches.iter().enumerate() {
            if !node_ids.contains(branch.to.as_str()) {
                bail!(
                    "node {}.branches[{index}] apunta a {} inexistente",
                    node.id,
                    branch.to
                );
            }
            validate_conditions(
                &format!("node {}.branches[{index}]", node.id),
                &branch.when,
                &document.variables,
            )?;
        }
        validate_effects(
            &format!("node {}.on_enter", node.id),
            &node.on_enter,
            &document.variables,
        )?;
        validate_effects(
            &format!("node {}.on_first_enter", node.id),
            &node.on_first_enter,
            &document.variables,
        )?;
        let mut choice_ids = HashSet::new();
        for choice in &node.choices {
            require_id(&format!("node {} choice.id", node.id), &choice.id)?;
            if !choice_ids.insert(choice.id.as_str()) {
                bail!("node {} repite choice id {}", node.id, choice.id);
            }
            require_text(
                &format!("node {} choice {}.label", node.id, choice.id),
                &choice.label,
            )?;
            if !node_ids.contains(choice.to.as_str()) {
                bail!(
                    "node {} choice {} apunta a {} inexistente",
                    node.id,
                    choice.id,
                    choice.to
                );
            }
            validate_conditions(
                &format!("node {} choice {}.visible_if", node.id, choice.id),
                &choice.visible_if,
                &document.variables,
            )?;
            validate_conditions(
                &format!("node {} choice {}.enabled_if", node.id, choice.id),
                &choice.enabled_if,
                &document.variables,
            )?;
            validate_effects(
                &format!("node {} choice {}.effects", node.id, choice.id),
                &choice.effects,
                &document.variables,
            )?;
        }
    }

    let reachable = reachable_nodes(document, &node_ids);
    // World-launched side VN blocks intentionally have no story edge from the
    // released prologue start.  They are entered by an interaction in the 3D
    // map, so keep them out of the linear-reachability warning while still
    // validating every node's schema, targets and assets above.
    let side_vn_ids: HashSet<&str> = document
        .nodes
        .iter()
        .filter(|node| {
            node.tags
                .iter()
                .any(|tag| tag == SIDE_VN_TAG || tag == OLD_PROLOGUE_TAG)
        })
        .map(|node| node.id.as_str())
        .collect();
    let mut unreachable: Vec<_> = node_ids
        .iter()
        .filter(|id| !reachable.contains(**id) && !side_vn_ids.contains(**id))
        .copied()
        .collect();
    unreachable.sort_unstable();
    let mut warnings = Vec::new();
    if !unreachable.is_empty() {
        warnings.push(format!(
            "nodos no alcanzables desde {}: {}",
            document.start,
            unreachable.join(", ")
        ));
    }
    Ok(warnings)
}

fn unique_ids<'a>(label: &str, ids: impl Iterator<Item = &'a str>) -> Result<HashSet<&'a str>> {
    let mut set = HashSet::new();
    for id in ids {
        require_id(label, id)?;
        if !set.insert(id) {
            bail!("{label} id duplicado: {id}");
        }
    }
    Ok(set)
}

fn require_id(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        bail!("{field} debe usar solo a-z, A-Z, 0-9, _ o -: {value:?}");
    }
    Ok(())
}

fn require_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} no puede estar vacío");
    }
    for marker in ["Ã", "Â", "â", "�"] {
        if value.contains(marker) {
            bail!("{field} contiene texto UTF-8 dañado ({marker:?})");
        }
    }
    Ok(())
}

fn validate_conditions(
    field: &str,
    conditions: &[ConditionSpec],
    variables: &BTreeMap<String, i64>,
) -> Result<()> {
    for condition in conditions {
        if !variables.contains_key(&condition.var) {
            bail!("{field} usa variable no declarada: {}", condition.var);
        }
    }
    Ok(())
}

fn validate_effects(
    field: &str,
    effects: &[EffectSpec],
    variables: &BTreeMap<String, i64>,
) -> Result<()> {
    for effect in effects {
        if !variables.contains_key(&effect.var) {
            bail!("{field} usa variable no declarada: {}", effect.var);
        }
    }
    Ok(())
}

fn reachable_nodes<'a>(
    document: &'a StoryDocument,
    node_ids: &HashSet<&'a str>,
) -> HashSet<&'a str> {
    let by_id: HashMap<_, _> = document
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut reached = HashSet::new();
    let mut queue = VecDeque::from([document.start.as_str()]);
    while let Some(id) = queue.pop_front() {
        if !reached.insert(id) {
            continue;
        }
        let Some(node) = by_id.get(id) else {
            continue;
        };
        if let Some(target) = node.next.as_deref() {
            if node_ids.contains(target) {
                queue.push_back(target);
            }
        }
        for choice in &node.choices {
            if node_ids.contains(choice.to.as_str()) {
                queue.push_back(choice.to.as_str());
            }
        }
    }
    reached
}

fn conditions_match(variables: &BTreeMap<String, i64>, conditions: &[ConditionSpec]) -> bool {
    conditions.iter().all(|condition| {
        let current = variables.get(&condition.var).copied().unwrap_or(0);
        match condition.op {
            ConditionOp::Eq => current == condition.value,
            ConditionOp::Ne => current != condition.value,
            ConditionOp::Gt => current > condition.value,
            ConditionOp::Gte => current >= condition.value,
            ConditionOp::Lt => current < condition.value,
            ConditionOp::Lte => current <= condition.value,
        }
    })
}

fn apply_effects(variables: &mut BTreeMap<String, i64>, effects: &[EffectSpec]) {
    for effect in effects {
        let current = variables.get(&effect.var).copied().unwrap_or(0);
        let next = match effect.op {
            EffectOp::Set => effect.value,
            EffectOp::Add => current.saturating_add(effect.value),
            EffectOp::Subtract => current.saturating_sub(effect.value),
            EffectOp::Min => current.min(effect.value),
            EffectOp::Max => current.max(effect.value),
        };
        variables.insert(effect.var.clone(), next);
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    // Written now: whatever the save writer still holds for it is older.
    crate::exploration3d_state::background::flush();
    crate::exploration3d_state::background::forget(path);
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("ruta sin carpeta padre: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("no se pudo crear {}", parent.display()))?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).context("no se pudo serializar el guardado")?;
    use std::io::Write;
    let mut output = fs::File::create(&temporary)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    drop(output);
    // rename replaces atomically (including Windows); never delete the old save first.
    fs::rename(&temporary, path).with_context(|| format!("no se pudo confirmar {}", path.display()))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small story with a crossroads after its first line: `x >= 1` goes to "alta", else
    /// on to "baja".
    fn crossroads_story() -> StoryRuntime {
        let json = serde_json::json!({
            "format": STORY_FORMAT, "schema": STORY_SCHEMA, "revision": 1, "id": "cruce",
            "title": "Cruce", "start": "a", "variables": { "x": 0 },
            "rooms": [{ "id": "sala", "name": "Sala", "background": "void", "description": "Una sala." }],
            "characters": [{ "id": "narrator", "name": "Narrador", "role": "voz", "visual_description": "—" }],
            "nodes": [
                { "id": "a", "room": "sala", "speaker": "narrator", "text": "Hola.", "visual": { "description": "a", "shot": "a" },
                  "choices": [
                    { "id": "sube", "label": "Subir", "to": "p", "effects": [{ "op": "set", "var": "x", "value": 1 }] },
                    { "id": "queda", "label": "Quedarse", "to": "p" }
                  ] },
                { "id": "p", "room": "sala", "speaker": "narrator", "text": "·", "visual": { "description": "p", "shot": "p" },
                  "pass": true, "next": "baja", "branches": [{ "when": [{ "var": "x", "op": "gte", "value": 1 }], "to": "alta" }] },
                { "id": "alta", "room": "sala", "speaker": "narrator", "text": "Arriba.", "visual": { "description": "x", "shot": "x" }, "ending": true },
                { "id": "baja", "room": "sala", "speaker": "narrator", "text": "Abajo.", "visual": { "description": "x", "shot": "x" }, "ending": true }
            ]
        });
        let document: StoryDocument = serde_json::from_value(json).unwrap();
        validate_document(&document).expect("a valid crossroads");
        let bytes = serde_json::to_vec(&document).unwrap();
        StoryRuntime::from_bytes(&bytes, "prueba").unwrap()
    }

    #[test]
    fn a_crossroads_leads_on_by_the_story_variables() {
        let story = crossroads_story();
        for (choice, expected) in [(0, "alta"), (1, "baja")] {
            let mut state = story.new_state().unwrap();
            assert_eq!(state.node, "a");
            story.decide(&mut state, choice).unwrap();
            assert_eq!(state.node, expected, "choice {choice}");
            assert!(
                state.visited.contains_key("p"),
                "the crossroads was passed through"
            );
        }
    }

    #[test]
    fn a_guion_vn_scene_takes_its_crossroads_by_the_story_variables() {
        // castillo.md, «luna»: after its first line, `@si seal_awake >= 1 -> luna_sello`.
        for (awake, expected) in [(0, "En mi otra vida"), (1, "Allá abajo")] {
            let mut game = Game::load().expect("story loads");
            game.new_game().unwrap();
            game.set_story_variables(&BTreeMap::from([("seal_awake".to_string(), awake)]));
            game.jump_to_node("g_castillo__luna_0")
                .expect("the guion's vn scene is in the story");
            let first = game.view().unwrap();
            assert!(first.text.starts_with("La luna"), "{}", first.text);
            game.decide(0).unwrap();
            let next = game.view().unwrap();
            assert!(
                next.text.starts_with(expected),
                "seal_awake {awake}: {}",
                next.text
            );
        }
    }

    #[test]
    fn a_crossroads_needs_a_way_on_and_no_choices() {
        let story = crossroads_story();
        let mut document = story.document.clone();
        document.nodes[1].next = None;
        assert!(validate_document(&document).is_err());
        let mut document = story.document.clone();
        document.nodes[0].branches = document.nodes[1].branches.clone();
        assert!(
            validate_document(&document).is_err(),
            "branches only on a crossroads"
        );
    }

    #[test]
    fn real_story_is_valid_and_every_node_has_an_image_brief() {
        let runtime = StoryRuntime::load(&story_path()).expect("valid JSON story");
        let summary = runtime.summary();
        assert_eq!(summary.id, "demon-queen-prologue");
        assert!(summary.rooms >= 4);
        assert!(summary.characters >= 8);
        assert!(summary.nodes >= 40);
        assert_eq!(summary.image_briefs, summary.nodes);
        assert!(summary.warnings.is_empty(), "{:?}", summary.warnings);
    }

    #[test]
    fn world_launched_mage_vn_returns_to_the_same_world_entry() {
        let mut game = Game::load().expect("story loads");
        game.new_game().unwrap();
        game.jump_to_node("mage_vn_01_intro").unwrap();
        assert!(
            !game.can_rollback(),
            "side VN must not expose prologue history"
        );
        let intro = game.view().unwrap();
        assert_eq!(intro.bg, "reference/approved/bg_forest_clearing");
        assert!(intro.image_asset.is_none());
        assert_eq!(intro.layers.len(), 2);

        for _ in 0..64 {
            let view = game.view().unwrap();
            if view.ending {
                assert_eq!(view.id, "mage_vn_20_unlock");
                assert_eq!(
                    view.world_entry.as_ref().map(|entry| entry.world.as_str()),
                    Some("act1_castle")
                );
                return;
            }
            let index = view
                .choices
                .iter()
                .position(|choice| choice.enabled)
                .expect("mage VN has an enabled continue/choice");
            game.decide(index).unwrap();
        }
        panic!("mage VN did not reach its return node");
    }

    #[test]
    fn world_launched_mage_vn_has_its_own_rollback_history() {
        let mut game = Game::load().expect("story loads");
        game.new_game().unwrap();
        game.jump_to_node("mage_vn_01_intro").unwrap();
        assert!(
            !game.can_rollback(),
            "side VN must not use prologue history"
        );

        let first = game.view().unwrap().id;
        let choice = game
            .view()
            .unwrap()
            .choices
            .iter()
            .position(|choice| choice.enabled)
            .expect("intro has a continue action");
        game.decide(choice).unwrap();
        assert_eq!(game.view().unwrap().id, "mage_vn_02_greeting");

        assert!(game.rollback_world_vn().unwrap());
        assert_eq!(game.view().unwrap().id, first);
        assert!(!game.can_rollback(), "rollback must stay out of prologue");
        assert!(!game.rollback_world_vn().unwrap());
    }

    #[test]
    fn story_runtime_preserves_spanish_utf8() {
        let source = fs::read_to_string(story_path()).expect("story is valid UTF-8");
        for marker in ["Ã", "Â", "â", "�"] {
            assert!(!source.contains(marker), "mojibake marker {marker:?}");
        }
        assert!(source.contains("Qué raro."));
        assert!(source.contains("pérdida de memoria"));
        assert!(source.contains("¿Quién trazó este sello?"));
    }

    #[test]
    fn every_branch_converges_on_the_prologue_end() {
        for route in 0..12usize {
            let mut game = Game::load().expect("story loads");
            game.new_game().unwrap();
            for step in 0..256 {
                let view = game.view().unwrap();
                if view.ending {
                    // Every way through the new prologue ends at Lira's «El Consejo espera».
                    assert!(
                        view.id.starts_with("g_prologo__p08_s8_consejo"),
                        "ending inesperado: {}",
                        view.id
                    );
                    assert_eq!(
                        view.world_entry.as_ref().map(|entry| entry.world.as_str()),
                        Some("act1_castle")
                    );
                    assert!(
                        view.text.contains("El Consejo espera"),
                        "texto de cierre inesperado: {}",
                        view.text
                    );
                    assert!(view.choices.is_empty());
                    break;
                }
                assert!(!view.choices.is_empty(), "dead end at {}", view.id);
                let enabled: Vec<_> = view
                    .choices
                    .iter()
                    .enumerate()
                    .filter(|(_, choice)| choice.enabled)
                    .map(|(index, _)| index)
                    .collect();
                assert!(!enabled.is_empty(), "no enabled choice at {}", view.id);
                game.decide(enabled[(route + step) % enabled.len()])
                    .unwrap();
                if step == 255 {
                    panic!("route {route} did not converge");
                }
            }
        }
    }

    #[test]
    fn a_save_in_the_old_prologue_starts_the_new_one_and_its_gate_still_opens() {
        let runtime = StoryRuntime::load(&story_path()).expect("story loads");
        let mut midway = runtime.new_state().unwrap();
        midway.node = "p03_ah".to_string();
        let migrated = runtime.migrate_state(midway).unwrap();
        assert_eq!(migrated.node, runtime.document.start);
        assert!(migrated.node.starts_with(PROLOGUE_NODE_PREFIX));

        let mut at_gate = runtime.new_state().unwrap();
        at_gate.node = "prologue_end".to_string();
        assert_eq!(runtime.migrate_state(at_gate).unwrap().node, "prologue_end");
    }

    #[test]
    fn current_release_redirects_legacy_act1_saves_to_the_prologue_gate() {
        let runtime = StoryRuntime::load(&story_path()).expect("story loads");
        let mut legacy = runtime.new_state().unwrap();
        legacy.node = "a1_11_pip_interrogation".to_string();

        let migrated = runtime.migrate_state(legacy).unwrap();
        assert_eq!(migrated.node, "prologue_end");
        let view = runtime.view(&migrated).unwrap();
        assert!(view.ending);
        assert!(view.choices.is_empty());
    }

    #[test]
    fn save_roundtrip_restores_node_variables_and_history() {
        let mut game = Game::load().unwrap();
        game.new_game().unwrap();
        for _ in 0..8 {
            let view = game.view().unwrap();
            if view.ending {
                break;
            }
            game.decide(0).unwrap();
        }
        let before = game.state.clone();
        let rollback_before = game.rollback.clone();
        let path = std::env::temp_dir().join(format!(
            "demon-queen-save-{}-{}.json",
            std::process::id(),
            now_unix()
        ));
        game.save_to_path(&path).unwrap();

        let mut restored = Game::load().unwrap();
        restored.load_from_path(&path).unwrap();
        assert_eq!(restored.state, before);
        assert_eq!(restored.rollback, rollback_before);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_roundtrip_preserves_exploration_position_and_seen_dialogues() {
        let mut game = Game::load().unwrap();
        game.new_game().unwrap();
        let mut world =
            crate::exploration3d_state::State::new("act1_castle", "bedroom_inside", 12.75, 15.125);
        world.yaw = std::f32::consts::FRAC_PI_4 * 3.0;
        world
            .seen_interactions
            .insert("talk_lira_intro".to_string());
        game.checkpoint_3d(world);
        let expected = ExplorationState {
            world: "act1_castle".to_string(),
            x_milli: 12_750,
            y_milli: 15_125,
            facing_x_milli: 707,
            facing_y_milli: -707,
            seen_interactions: BTreeSet::from(["talk_lira_intro".to_string()]),
        };
        assert_eq!(game.exploration_state(), Some(&expected));
        let path = std::env::temp_dir().join(format!(
            "demon-queen-exploration-save-{}-{}.json",
            std::process::id(),
            now_unix()
        ));
        game.save_to_path(&path).unwrap();

        let mut restored = Game::load().unwrap();
        restored.load_from_path(&path).unwrap();
        assert_eq!(restored.exploration_state(), Some(&expected));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rollback_restores_previous_scene_and_survives_branch_effects() {
        let mut game = Game::load().unwrap();
        game.new_game().unwrap();
        let opening = game.view().unwrap();
        assert!(!game.can_rollback());

        game.decide(0).unwrap();
        assert!(game.can_rollback());
        assert_ne!(game.view().unwrap().id, opening.id);

        assert!(game.rollback().unwrap());
        assert_eq!(game.view().unwrap().id, opening.id);
        assert!(!game.can_rollback());
        assert!(!game.rollback().unwrap());
    }

    #[test]
    fn invalid_target_is_rejected_with_context() {
        let bytes = fs::read(story_path()).unwrap();
        let mut document: StoryDocument = serde_json::from_slice(&bytes).unwrap();
        document.nodes[0].next = Some("missing-node".to_string());
        document.nodes[0].choices.clear();
        let error = validate_document(&document).unwrap_err().to_string();
        assert!(error.contains("missing-node"));
        assert!(error.contains("inexistente"));
    }

    #[test]
    fn manual_slot_overwrite_keeps_backup_and_embeds_world_state() {
        let mut game = Game::load().unwrap();
        game.new_game().unwrap();
        game.decide(0).unwrap();
        let mut world =
            crate::exploration3d_state::State::new("act1_castle", "bedroom_door", 2.125, -7.5);
        world.nature_mode = true;
        world.village_entered = true;
        world.outfit = 1;
        world.yaw = 1.2;
        world.seen_interactions.insert("door".into());
        game.checkpoint_3d(world.clone());
        let path = std::env::temp_dir().join(format!("dq-manual-{}.json", std::process::id()));
        game.save_manual_to_path(&path, Some(vec![1, 2, 3]))
            .unwrap();
        let original = fs::read(&path).unwrap();
        game.decide(0).unwrap();
        game.save_manual_to_path(&path, None).unwrap();
        assert_eq!(fs::read(path.with_extension("json.bak")).unwrap(), original);
        let mut loaded = Game::load().unwrap();
        loaded.load_from_path(&path).unwrap();
        assert_eq!(loaded.state, game.state);
        assert_eq!(loaded.rollback, game.rollback);
        assert_eq!(loaded.world_snapshot(), Some(&world));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.bak"));
    }

    #[test]
    fn legacy_save_loads_without_new_optional_fields() {
        let game = Game::load().unwrap();
        let mut json = serde_json::to_value(game.save_file()).unwrap();
        json.as_object_mut().unwrap().remove("world3d");
        json.as_object_mut().unwrap().remove("thumbnail");
        let saved: SaveFile = serde_json::from_value(json).unwrap();
        game.validate_save(&saved).unwrap();
        assert!(saved.world3d.is_none() && saved.thumbnail.is_none());
    }

    #[test]
    fn corrupt_save_does_not_mutate_active_game() {
        let mut game = Game::load().unwrap();
        game.decide(0).unwrap();
        let previous = game.state.clone();
        let path =
            std::env::temp_dir().join(format!("dq-invalid-slot-{}.json", std::process::id()));
        fs::write(&path, b"{truncated}").unwrap();
        assert!(game.load_from_path(&path).is_err());
        assert_eq!(game.state, previous);
        assert!(slot_path(MANUAL_SLOTS).is_err());
        assert!(slot_path(usize::MAX).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn viewing_history_is_read_only() {
        let mut game = Game::load().unwrap();
        game.decide(0).unwrap();
        game.decide(0).unwrap();
        let previous = game.state.clone();
        let views = game.history_views();
        assert_eq!(views.len(), 3);
        assert_eq!(views.last().unwrap().id, previous.node);
        assert_eq!(game.state, previous);
    }

    #[test]
    fn editor_schema_is_valid_json() {
        let schema = story_path().with_file_name("schema.json");
        let bytes = fs::read(&schema).expect("story/schema.json exists");
        let document: serde_json::Value =
            serde_json::from_slice(&bytes).expect("story/schema.json is valid JSON");
        assert_eq!(
            document.get("title").and_then(serde_json::Value::as_str),
            Some("Velvet Story v1")
        );
    }

    #[test]
    fn storyboard_places_one_image_brief_after_every_dialogue() {
        let runtime = StoryRuntime::load(&story_path()).expect("valid JSON story");
        let markdown = runtime.storyboard_markdown();
        // Every node that is shown (crossroads are not) has its brief.
        assert_eq!(
            markdown.matches("> **Ficha de imagen**").count(),
            runtime
                .document
                .nodes
                .iter()
                .filter(|node| !node.pass)
                .count()
        );
        assert!(markdown.contains("**Reina original:**"));
        assert!(markdown.contains("**Selecciones:**"));
    }

    #[test]
    fn gallery_deduplicates_art_and_excludes_locked_act_one() {
        let runtime = StoryRuntime::load(&story_path()).expect("valid JSON story");
        let unlocked: BTreeSet<String> = runtime
            .document
            .nodes
            .iter()
            .filter_map(|node| gallery_asset(node).map(str::to_owned))
            .collect();
        let library = runtime.gallery_library(&unlocked);
        let assets: BTreeSet<_> = library
            .entries
            .iter()
            .map(|entry| entry.asset.as_str())
            .collect();

        assert!(library.total >= 70);
        assert_eq!(library.entries.len(), library.total);
        assert_eq!(assets.len(), library.entries.len());
        assert!(library
            .entries
            .iter()
            .all(|entry| !entry.node_id.starts_with("a1_")));
        // The pictures are the new prologue's (its `@cg`s); the old prologue's are out.
        assert!(library
            .entries
            .iter()
            .all(|entry| entry.node_id.starts_with(PROLOGUE_NODE_PREFIX)));
    }
}
