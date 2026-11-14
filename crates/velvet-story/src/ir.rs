//! Story intermediate representation (linear ops per scene).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::value::StoryValue;
use crate::variables::AssignOp;

/// Mini-ATL motion attached to a `show` (parsed from `with ...`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowMotion {
    /// Canonical kind: `move` | `zoom` | `shake` | `fade`.
    pub kind: String,
    /// Duration seconds.
    pub duration_secs: f32,
    /// Amount (zoom scale add, shake px, else 0).
    pub amount: f32,
}

impl ShowMotion {
    /// New motion.
    pub fn new(kind: impl Into<String>, duration_secs: f32, amount: f32) -> Self {
        Self {
            kind: kind.into(),
            duration_secs: duration_secs.clamp(0.05, 3.0),
            amount: amount.clamp(0.0, 64.0),
        }
    }
}

/// One executable story operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryOp {
    /// Set background image path.
    Background {
        /// Asset path.
        path: String,
    },
    /// Play music.
    Music {
        /// Path.
        path: String,
        /// Fade-in seconds.
        fade_in: Option<f64>,
    },
    /// Show character / sprite.
    Show {
        /// Target e.g. `aria.neutral`.
        target: String,
        /// Placement e.g. `left` / `right` / `center`.
        at: Option<String>,
        /// Optional mini-ATL motion from `with ...`.
        #[serde(default)]
        motion: Option<ShowMotion>,
        /// Named ATL transform ref (`show … with bob`).
        #[serde(default)]
        transform: Option<String>,
    },
    /// Apply a VN screen preset (`screen say_classic` — 3-line authoring).
    Screen {
        /// Preset id (`say_classic` / `say_minimal` / `nvl_page` / `menu`).
        id: String,
    },
    /// Show a declarative game screen (`screen show inv (…)`).
    ShowGameScreen {
        /// Screen id.
        id: String,
        /// Positional argument values.
        #[serde(default)]
        args: Vec<StoryValue>,
        /// Enter mini-ATL motion (`with …`).
        #[serde(default)]
        motion: Option<ShowMotion>,
        /// Enter named ATL transform (`with <name>`).
        #[serde(default)]
        transform: Option<String>,
    },
    /// Hide a declarative game screen (`screen hide inv`).
    HideGameScreen {
        /// Screen id.
        id: String,
        /// Exit mini-ATL motion (`with …`).
        #[serde(default)]
        motion: Option<ShowMotion>,
        /// Exit named ATL transform (`with <name>`).
        #[serde(default)]
        transform: Option<String>,
    },
    /// Call a game screen modally (`screen call inv (…)` — blocks for
    /// [`crate::runtime::StoryPlayer::return_from_screen`]).
    CallGameScreen {
        /// Screen id.
        id: String,
        /// Positional argument values.
        #[serde(default)]
        args: Vec<StoryValue>,
        /// Enter mini-ATL motion (`with …`).
        #[serde(default)]
        motion: Option<ShowMotion>,
        /// Enter named ATL transform (`with <name>`).
        #[serde(default)]
        transform: Option<String>,
        /// Result variable (`into <var>`, in addition to `_return`).
        #[serde(default)]
        result: Option<String>,
    },
    /// Unlock a gallery CG or music-room track (`unlock gallery <id>`).
    Unlock {
        /// Catalog: `gallery` or `music`.
        catalog: String,
        /// Entry id.
        id: String,
    },
    /// Play a cinema clip (`movie "assets/video/opening" [loop]`).
    Movie {
        /// Clip directory path (project-relative).
        path: String,
        /// Loop instead of finishing.
        loop_clip: bool,
    },
    /// Hide character.
    Hide {
        /// Target id.
        target: String,
        /// Optional mini-ATL motion from `with ...`.
        #[serde(default)]
        motion: Option<ShowMotion>,
        /// Named ATL transform ref (`hide … with bob`).
        #[serde(default)]
        transform: Option<String>,
    },
    /// Dialogue or monologue.
    Dialogue {
        /// Speaker character id (None = narrator).
        speaker: Option<String>,
        /// Raw text (may contain `{vars}`).
        text: String,
    },
    /// Present choices; each arm is a list of ops (inline) or ends with Jump.
    Choice {
        /// Options.
        options: Vec<StoryChoice>,
    },
    /// Jump to scene name or label `scene:label`.
    Jump {
        /// Target.
        target: String,
    },
    /// Call scene as subroutine (return later) — v1 maps to jump.
    Call {
        /// Target scene.
        target: String,
    },
    /// Label within scene.
    Label {
        /// Name.
        name: String,
    },
    /// Variable assignment.
    Assign {
        /// Variable name.
        name: String,
        /// Operator.
        assign_op: AssignOp,
        /// Right-hand expression (literal, variable, or simple arithmetic).
        value: StoryExpr,
    },
    /// Conditional block with a full narrative condition (vars, not/and/or, compares).
    If {
        /// Condition to evaluate at runtime.
        cond: StoryCond,
        /// Ops if true.
        then_ops: Vec<StoryOp>,
        /// Ops if false.
        else_ops: Vec<StoryOp>,
    },
    /// End the story with optional ending id.
    End {
        /// Ending id / name.
        ending: Option<String>,
    },
    /// Registered host command from Velvet Story (`call combat.start: …`).
    HostCall {
        /// Command name e.g. `combat.start`.
        name: String,
        /// Named arguments (primitives only).
        #[serde(default)]
        args: IndexMap<String, StoryValue>,
    },
    /// Play one-shot SFX.
    Sound {
        /// Asset path / id.
        path: String,
    },
    /// Pause / wait beat (seconds if known).
    Pause {
        /// Optional duration in seconds.
        seconds: Option<f64>,
    },
    /// Named transition (fade, dissolve, …).
    Transition {
        /// Transition id.
        name: String,
    },
    /// Jump to the scene the story's logic function answers with (`goto logic.name`); an empty
    /// or non-text answer, or one that names no scene or label, goes on with the next op.
    JumpLogic {
        /// Function of the story's logic file.
        function: String,
    },
    /// Return from a Call scene (pops call stack).
    Return,
    /// No-op / pause beat.
    Nop,
}

/// Arithmetic / value expression for assignment RHS (product IR).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryExpr {
    /// Immediate literal.
    Value {
        /// Literal value.
        value: StoryValue,
    },
    /// Load a play variable.
    Var {
        /// Variable name.
        name: String,
    },
    /// Binary arithmetic (`+ - * /`).
    Binary {
        /// Operator.
        op: StoryArithOp,
        /// Left.
        left: Box<StoryExpr>,
        /// Right.
        right: Box<StoryExpr>,
    },
    /// Unary numeric negation.
    Neg {
        /// Inner.
        inner: Box<StoryExpr>,
    },
    /// What a function of the story's logic file answers (`logic.name`): it is called with a map
    /// of the story's variables.
    Logic {
        /// Function name.
        function: String,
        /// Read a text answer as a label ([`label_slug`]): lower case, accents off, words joined
        /// by `_`, so `" Otro.Escena "` and `"otro_escena"` are the same answer. Guiones compile
        /// `@decide` with it, to match the labels after `->` as a conversation does.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        as_label: bool,
    },
}

/// A label as an identifier: lower case, accents off, words joined by `_`, and an `s` before a
/// leading digit. The same rule as velvet-guion's `slug`, which names guion scenes and labels.
pub fn label_slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' => 'u',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("s{out}")
    } else {
        out
    }
}

impl StoryExpr {
    /// Literal value.
    pub fn value(v: StoryValue) -> Self {
        Self::Value { value: v }
    }

    /// Variable reference.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var { name: name.into() }
    }
}

/// Arithmetic operator for [`StoryExpr::Binary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryArithOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
}

/// Operand for comparisons in [`StoryCond`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryOperand {
    /// Load a play variable.
    Var {
        /// Variable name.
        name: String,
    },
    /// Immediate literal value.
    Value {
        /// Literal.
        value: StoryValue,
    },
    /// What a function of the story's logic file answers (`logic.name`).
    Logic {
        /// Function name.
        function: String,
    },
}

/// Comparison operator for narrative conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryCmpOp {
    /// ==
    Eq,
    /// !=
    Ne,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
}

/// Canonical condition tree for [`StoryOp::If`] (product runtime).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryCond {
    /// Truthiness of a variable.
    Var {
        /// Variable name.
        name: String,
    },
    /// Constant boolean (folded literals).
    Const {
        /// Literal value.
        value: bool,
    },
    /// Logical not.
    Not {
        /// Inner condition.
        inner: Box<StoryCond>,
    },
    /// Logical and (short-circuit at runtime).
    And {
        /// Left.
        left: Box<StoryCond>,
        /// Right.
        right: Box<StoryCond>,
    },
    /// Logical or (short-circuit at runtime).
    Or {
        /// Left.
        left: Box<StoryCond>,
        /// Right.
        right: Box<StoryCond>,
    },
    /// Ordered / equality comparison.
    Cmp {
        /// Left operand.
        left: StoryOperand,
        /// Operator.
        op: StoryCmpOp,
        /// Right operand.
        right: StoryOperand,
    },
    /// Truthiness of what a function of the story's logic file answers (`if logic.name:`).
    Logic {
        /// Function name.
        function: String,
    },
}

impl StoryCond {
    /// Convenience: truthiness of a single variable name.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var { name: name.into() }
    }
}

impl StoryOperand {
    /// Variable operand.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var { name: name.into() }
    }

    /// Literal operand.
    pub fn value(v: StoryValue) -> Self {
        Self::Value { value: v }
    }
}

/// One choice arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryChoice {
    /// Display text.
    pub text: String,
    /// Ops executed when selected (before continuing).
    pub body: Vec<StoryOp>,
    /// Optional condition variable that must be truthy.
    pub require: Option<String>,
    /// If true, option hidden when require fails (vs locked).
    pub hidden_if_locked: bool,
}

/// Compiled scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryScene {
    /// Scene name.
    pub name: String,
    /// Operations in order.
    pub ops: Vec<StoryOp>,
    /// Label name → op index.
    #[serde(default)]
    pub labels: IndexMap<String, usize>,
}

impl StoryScene {
    /// Build label map from ops.
    pub fn reindex_labels(&mut self) {
        self.labels.clear();
        for (i, op) in self.ops.iter().enumerate() {
            if let StoryOp::Label { name } = op {
                self.labels.insert(name.clone(), i);
            }
        }
    }
}

/// Full story program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryProgram {
    /// Optional title.
    pub title: String,
    /// Characters by id.
    pub characters: IndexMap<String, crate::character::Character>,
    /// Initial variable values.
    pub initial_vars: IndexMap<String, StoryValue>,
    /// Scenes by name.
    pub scenes: IndexMap<String, StoryScene>,
    /// Declarative game screens by name (`screen inv(…)`).
    #[serde(default)]
    pub screens: IndexMap<String, crate::screens::ScreenDef>,
    /// Named ATL transforms (`transform bob { … }`).
    #[serde(default)]
    pub transforms: IndexMap<String, crate::atl::TransformDef>,
    /// Entry scene name.
    pub entry: String,
    /// The story's logic file (Velvet Script 3), whose functions `logic.name` calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logic: Option<StoryLogic>,
}

/// A story's logic file: Velvet Script 3 (`// @edition 3`). Each `logic.name` of the story calls
/// its function `name` with one argument, a map of the story's variables.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryLogic {
    /// Where it was read from (for messages).
    pub path: String,
    /// Its source.
    pub source: String,
}

impl StoryProgram {
    /// Stable content hash for save/load identity checks (sha256 hex).
    ///
    /// Covers entry, title, scenes (ops/labels), and initial vars — enough to
    /// detect when a save is reopened against a different narrative program.
    /// The logic file is left out: a save keeps its place in the scenes and the story's
    /// variables, which editing the logic file does not move, and where the file was read from
    /// says nothing about the story.
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        // JSON is deterministic enough for IndexMap (insertion order) + serde.
        let payload = serde_json::json!({
            "entry": &self.entry,
            "title": &self.title,
            "scenes": &self.scenes,
            "screens": &self.screens,
            "transforms": &self.transforms,
            "initial_vars": &self.initial_vars,
        });
        let bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let hash = Sha256::digest(&bytes);
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Create empty program.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            characters: IndexMap::new(),
            initial_vars: IndexMap::new(),
            scenes: IndexMap::new(),
            screens: IndexMap::new(),
            transforms: IndexMap::new(),
            entry: "main".into(),
            logic: None,
        }
    }

    /// Get scene.
    pub fn scene(&self, name: &str) -> Option<&StoryScene> {
        self.scenes.get(name)
    }
}
