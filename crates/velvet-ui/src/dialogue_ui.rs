//! Higher-level dialogue box controller (string/typewriter friendly).

use crate::animation::{Ease, UiTween};
use crate::style::UiStyle;

/// Dialogue box visual state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogueBoxPhase {
    /// Hidden.
    #[default]
    Hidden,
    /// Opening animation.
    Opening,
    /// Showing text (typewriter may still run externally).
    Open,
    /// Waiting for advance input.
    Waiting,
    /// Closing animation.
    Closing,
}

/// Controller for a narrative dialogue box.
#[derive(Debug, Clone)]
pub struct DialogueBox {
    /// Speaker name (optional).
    pub speaker: Option<String>,
    /// Full line text (markup allowed; display is external).
    pub full_text: String,
    /// Currently visible text (set by game/typewriter).
    pub visible_text: String,
    /// Phase.
    pub phase: DialogueBoxPhase,
    /// Panel style.
    pub style: UiStyle,
    /// Opacity for fade.
    pub opacity: f32,
    /// Vertical offset for slide-in (pixels).
    pub offset_y: f32,
    /// Open/close tween duration.
    pub anim_secs: f32,
    /// Internal open tween elapsed.
    anim_t: f32,
    /// Whether text reveal is complete.
    pub text_complete: bool,
    /// Choices currently offered.
    pub choices: Vec<String>,
    /// Selected choice index.
    pub selected_choice: usize,
}

impl Default for DialogueBox {
    fn default() -> Self {
        Self {
            speaker: None,
            full_text: String::new(),
            visible_text: String::new(),
            phase: DialogueBoxPhase::Hidden,
            style: UiStyle::dialogue_panel(),
            opacity: 0.0,
            offset_y: 40.0,
            anim_secs: 0.25,
            anim_t: 0.0,
            text_complete: false,
            choices: Vec::new(),
            selected_choice: 0,
        }
    }
}

impl DialogueBox {
    /// Create hidden box.
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a line with optional speaker; starts opening animation.
    pub fn show_line(&mut self, speaker: Option<String>, text: impl Into<String>) {
        self.speaker = speaker;
        self.full_text = text.into();
        self.visible_text.clear();
        self.text_complete = false;
        self.choices.clear();
        self.selected_choice = 0;
        self.phase = DialogueBoxPhase::Opening;
        self.anim_t = 0.0;
        self.opacity = 0.0;
        self.offset_y = 40.0;
    }

    /// Show choices (after or with a line).
    pub fn set_choices(&mut self, choices: impl IntoIterator<Item = impl Into<String>>) {
        self.choices = choices.into_iter().map(|c| c.into()).collect();
        self.selected_choice = 0;
        if self.phase == DialogueBoxPhase::Open || self.phase == DialogueBoxPhase::Waiting {
            self.phase = DialogueBoxPhase::Waiting;
        }
    }

    /// Sync visible text from an external typewriter string.
    pub fn set_visible_text(&mut self, text: impl Into<String>, complete: bool) {
        self.visible_text = text.into();
        self.text_complete = complete;
        if complete && self.phase == DialogueBoxPhase::Open {
            self.phase = DialogueBoxPhase::Waiting;
        }
    }

    /// Skip to full text.
    pub fn reveal_all(&mut self) {
        self.visible_text = self.full_text.clone();
        self.text_complete = true;
        if matches!(
            self.phase,
            DialogueBoxPhase::Open | DialogueBoxPhase::Opening
        ) {
            self.phase = DialogueBoxPhase::Waiting;
        }
    }

    /// Advance: if text incomplete, reveal all; if waiting, close or return choice.
    pub fn advance(&mut self) -> DialogueAdvance {
        match self.phase {
            DialogueBoxPhase::Hidden | DialogueBoxPhase::Closing => DialogueAdvance::None,
            DialogueBoxPhase::Opening | DialogueBoxPhase::Open => {
                if !self.text_complete {
                    self.reveal_all();
                    DialogueAdvance::RevealedAll
                } else {
                    self.begin_close();
                    DialogueAdvance::Closed
                }
            }
            DialogueBoxPhase::Waiting => {
                if !self.choices.is_empty() {
                    let idx = self.selected_choice.min(self.choices.len() - 1);
                    let label = self.choices[idx].clone();
                    self.begin_close();
                    DialogueAdvance::Choice { index: idx, label }
                } else {
                    self.begin_close();
                    DialogueAdvance::Closed
                }
            }
        }
    }

    fn begin_close(&mut self) {
        self.phase = DialogueBoxPhase::Closing;
        self.anim_t = 0.0;
    }

    /// Move choice selection.
    pub fn move_choice(&mut self, delta: i32) {
        if self.choices.is_empty() {
            return;
        }
        let n = self.choices.len() as i32;
        let cur = self.selected_choice as i32 + delta;
        self.selected_choice = ((cur % n + n) % n) as usize;
    }

    /// Tick open/close animation.
    pub fn tick(&mut self, dt: f32) {
        match self.phase {
            DialogueBoxPhase::Opening => {
                self.anim_t += dt;
                let t = (self.anim_t / self.anim_secs).clamp(0.0, 1.0);
                let e = Ease::EaseOutCubic.eval(t);
                self.opacity = e;
                self.offset_y = 40.0 * (1.0 - e);
                if t >= 1.0 {
                    self.phase = DialogueBoxPhase::Open;
                    self.opacity = 1.0;
                    self.offset_y = 0.0;
                }
            }
            DialogueBoxPhase::Closing => {
                self.anim_t += dt;
                let t = (self.anim_t / self.anim_secs).clamp(0.0, 1.0);
                let e = Ease::EaseInCubic.eval(t);
                self.opacity = 1.0 - e;
                self.offset_y = 20.0 * e;
                if t >= 1.0 {
                    self.phase = DialogueBoxPhase::Hidden;
                    self.opacity = 0.0;
                    self.full_text.clear();
                    self.visible_text.clear();
                    self.choices.clear();
                }
            }
            _ => {}
        }
    }

    /// Whether visible enough to draw.
    pub fn is_visible(&self) -> bool {
        !matches!(self.phase, DialogueBoxPhase::Hidden) && self.opacity > 0.01
    }

    /// Helper open tween sample (for external systems).
    pub fn open_tween() -> UiTween {
        UiTween::opacity(0.0, 1.0, 0.25)
    }
}

/// Result of [`DialogueBox::advance`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueAdvance {
    /// No-op.
    None,
    /// Skipped typewriter.
    RevealedAll,
    /// Box closed after line.
    Closed,
    /// Player selected a choice.
    Choice {
        /// Index.
        index: usize,
        /// Label.
        label: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_and_reveal() {
        let mut d = DialogueBox::new();
        d.show_line(Some("Ava".into()), "Hello there.");
        d.tick(0.3);
        assert!(d.is_visible());
        assert_eq!(d.phase, DialogueBoxPhase::Open);
        d.set_visible_text("Hello", false);
        let r = d.advance();
        assert_eq!(r, DialogueAdvance::RevealedAll);
        assert_eq!(d.visible_text, "Hello there.");
    }

    #[test]
    fn choices() {
        let mut d = DialogueBox::new();
        d.show_line(None, "Pick one");
        d.tick(1.0);
        d.reveal_all();
        d.set_choices(["Yes", "No"]);
        d.move_choice(1);
        assert_eq!(d.selected_choice, 1);
        let r = d.advance();
        assert!(matches!(r, DialogueAdvance::Choice { index: 1, .. }));
    }
}
