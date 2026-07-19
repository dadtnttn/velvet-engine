//! Story intermediate representation (linear ops per scene).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::value::StoryValue;
use crate::variables::AssignOp;

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
    },
    /// Hide character.
    Hide {
        /// Target id.
        target: String,
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
    /// Entry scene name.
    pub entry: String,
}

impl StoryProgram {
    /// Stable content hash for save/load identity checks (sha256 hex).
    ///
    /// Covers entry, title, scenes (ops/labels), and initial vars — enough to
    /// detect when a save is reopened against a different narrative program.
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        // JSON is deterministic enough for IndexMap (insertion order) + serde.
        let payload = serde_json::json!({
            "entry": &self.entry,
            "title": &self.title,
            "scenes": &self.scenes,
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
            entry: "main".into(),
        }
    }

    /// Get scene.
    pub fn scene(&self, name: &str) -> Option<&StoryScene> {
        self.scenes.get(name)
    }
}
