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
        /// Right-hand value.
        value: StoryValue,
    },
    /// Conditional block (simple).
    If {
        /// Variable name to test truthiness.
        cond_var: String,
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
