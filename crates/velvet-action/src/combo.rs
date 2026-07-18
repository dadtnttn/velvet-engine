//! Attack combo chain with per-step timing windows.

use serde::{Deserialize, Serialize};

/// One step in an attack combo chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComboStep {
    /// Step id / animation name.
    pub id: String,
    /// Damage multiplier for this step.
    pub damage_mul: f32,
    /// Active frames duration (seconds) where hitboxes are live.
    pub active_secs: f32,
    /// Recovery duration after active (seconds).
    pub recovery_secs: f32,
    /// Window after recovery starts during which the next input queues the next step.
    pub link_window_secs: f32,
    /// Optional cancel into special (seconds from step start).
    pub special_cancel_secs: Option<f32>,
}

impl ComboStep {
    /// Create a step.
    pub fn new(
        id: impl Into<String>,
        damage_mul: f32,
        active: f32,
        recovery: f32,
        link: f32,
    ) -> Self {
        Self {
            id: id.into(),
            damage_mul,
            active_secs: active.max(0.0),
            recovery_secs: recovery.max(0.0),
            link_window_secs: link.max(0.0),
            special_cancel_secs: None,
        }
    }

    /// Total duration of the step.
    pub fn total_secs(&self) -> f32 {
        self.active_secs + self.recovery_secs
    }
}

/// Phase of the current combo step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComboPhase {
    /// Idle / not attacking.
    #[default]
    Idle,
    /// Startup/active hit frames.
    Active,
    /// Recovery; may accept link input.
    Recovery,
}

/// Input event for the combo system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboInput {
    /// Light / primary attack.
    Attack,
    /// Special cancel request.
    Special,
}

/// Result of feeding input / ticking.
#[derive(Debug, Clone, PartialEq)]
pub enum ComboEvent {
    /// Started a step.
    StepStarted {
        /// Index in the chain.
        index: usize,
        /// Step id.
        id: String,
        /// Damage multiplier.
        damage_mul: f32,
    },
    /// Entered recovery of current step.
    EnteredRecovery {
        /// Index.
        index: usize,
    },
    /// Combo finished and returned to idle.
    ComboEnded {
        /// Steps completed.
        steps: usize,
    },
    /// Special cancel succeeded.
    SpecialCancel {
        /// From step index.
        from_index: usize,
    },
}

/// Attack combo chain state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttackCombo {
    /// Ordered chain of steps.
    pub steps: Vec<ComboStep>,
    /// Current step index (valid when not idle).
    pub index: usize,
    /// Current phase.
    pub phase: ComboPhase,
    /// Elapsed time in the current phase.
    pub phase_elapsed: f32,
    /// Queued link to next step.
    pub link_queued: bool,
    /// Steps completed in this chain (for score).
    pub steps_done: usize,
}

impl Default for AttackCombo {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            index: 0,
            phase: ComboPhase::Idle,
            phase_elapsed: 0.0,
            link_queued: false,
            steps_done: 0,
        }
    }
}

impl AttackCombo {
    /// Create from steps.
    pub fn new(steps: Vec<ComboStep>) -> Self {
        Self {
            steps,
            ..Default::default()
        }
    }

    /// Classic 3-hit light combo.
    pub fn light_chain_3() -> Self {
        Self::new(vec![
            ComboStep::new("light_1", 1.0, 0.08, 0.18, 0.2),
            ComboStep::new("light_2", 1.1, 0.08, 0.2, 0.22),
            ComboStep::new("light_3", 1.35, 0.12, 0.28, 0.0).with_special_cancel(0.1),
        ])
    }

    /// Whether currently mid-combo.
    pub fn is_busy(&self) -> bool {
        self.phase != ComboPhase::Idle
    }

    /// Current step if any.
    pub fn current_step(&self) -> Option<&ComboStep> {
        if self.phase == ComboPhase::Idle {
            None
        } else {
            self.steps.get(self.index)
        }
    }

    /// Damage multiplier of the active step (1.0 if idle).
    pub fn damage_mul(&self) -> f32 {
        self.current_step().map(|s| s.damage_mul).unwrap_or(1.0)
    }

    /// Whether hitboxes should be live.
    pub fn is_active_frames(&self) -> bool {
        self.phase == ComboPhase::Active
    }

    /// Feed an input. Returns events.
    pub fn input(&mut self, input: ComboInput) -> Vec<ComboEvent> {
        let mut events = Vec::new();
        match input {
            ComboInput::Attack => match self.phase {
                ComboPhase::Idle => {
                    if !self.steps.is_empty() {
                        self.start_step(0, &mut events);
                    }
                }
                ComboPhase::Active => {
                    // Buffer link during active if we want; queue for recovery.
                    self.link_queued = true;
                }
                ComboPhase::Recovery => {
                    if let Some(step) = self.steps.get(self.index) {
                        if self.phase_elapsed <= step.link_window_secs {
                            self.link_queued = true;
                        }
                    }
                }
            },
            ComboInput::Special => {
                if let Some(step) = self.current_step() {
                    if let Some(window) = step.special_cancel_secs {
                        let total_elapsed = match self.phase {
                            ComboPhase::Active => self.phase_elapsed,
                            ComboPhase::Recovery => step.active_secs + self.phase_elapsed,
                            ComboPhase::Idle => 0.0,
                        };
                        if total_elapsed <= window || self.phase == ComboPhase::Active {
                            let from = self.index;
                            self.reset_idle();
                            events.push(ComboEvent::SpecialCancel { from_index: from });
                        }
                    }
                }
            }
        }
        events
    }

    /// Tick the combo. Returns events.
    pub fn tick(&mut self, dt: f32) -> Vec<ComboEvent> {
        let mut events = Vec::new();
        let dt = dt.max(0.0);
        if self.phase == ComboPhase::Idle || self.steps.is_empty() {
            return events;
        }
        let step = self.steps[self.index].clone();
        self.phase_elapsed += dt;
        match self.phase {
            ComboPhase::Active => {
                if self.phase_elapsed >= step.active_secs {
                    self.phase = ComboPhase::Recovery;
                    self.phase_elapsed = 0.0;
                    events.push(ComboEvent::EnteredRecovery { index: self.index });
                }
            }
            ComboPhase::Recovery => {
                // Accept link during window
                let can_link = self.phase_elapsed <= step.link_window_secs || self.link_queued;
                if self.link_queued && can_link && self.index + 1 < self.steps.len() {
                    let next = self.index + 1;
                    self.link_queued = false;
                    self.start_step(next, &mut events);
                    return events;
                }
                if self.phase_elapsed >= step.recovery_secs {
                    // End of step — try link if queued early
                    if self.link_queued && self.index + 1 < self.steps.len() {
                        // only if still within grace after recovery ends? drop if late
                        let next = self.index + 1;
                        self.link_queued = false;
                        self.start_step(next, &mut events);
                    } else {
                        let steps = self.steps_done;
                        self.reset_idle();
                        events.push(ComboEvent::ComboEnded { steps });
                    }
                }
            }
            ComboPhase::Idle => {}
        }
        events
    }

    fn start_step(&mut self, index: usize, events: &mut Vec<ComboEvent>) {
        self.index = index;
        self.phase = ComboPhase::Active;
        self.phase_elapsed = 0.0;
        self.link_queued = false;
        self.steps_done = index + 1;
        let step = &self.steps[index];
        events.push(ComboEvent::StepStarted {
            index,
            id: step.id.clone(),
            damage_mul: step.damage_mul,
        });
    }

    fn reset_idle(&mut self) {
        self.phase = ComboPhase::Idle;
        self.phase_elapsed = 0.0;
        self.link_queued = false;
        self.index = 0;
        self.steps_done = 0;
    }

    /// Force cancel to idle.
    pub fn cancel(&mut self) {
        self.reset_idle();
    }
}

impl ComboStep {
    /// Builder: special cancel window.
    pub fn with_special_cancel(mut self, secs: f32) -> Self {
        self.special_cancel_secs = Some(secs.max(0.0));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_chain_with_links() {
        let mut c = AttackCombo::light_chain_3();
        let e = c.input(ComboInput::Attack);
        assert!(matches!(e[0], ComboEvent::StepStarted { index: 0, .. }));
        // Finish active
        let _ = c.tick(0.1);
        assert_eq!(c.phase, ComboPhase::Recovery);
        // Link during window
        let _ = c.input(ComboInput::Attack);
        let e2 = c.tick(0.01);
        assert!(e2
            .iter()
            .any(|ev| matches!(ev, ComboEvent::StepStarted { index: 1, .. })));
        // Finish step 2 and link to 3
        let _ = c.tick(0.1); // active -> recovery
        let _ = c.input(ComboInput::Attack);
        let _ = c.tick(0.01);
        assert_eq!(c.index, 2);
        // Finish last without link
        let _ = c.tick(0.2);
        let e3 = c.tick(0.5);
        assert!(e3
            .iter()
            .any(|ev| matches!(ev, ComboEvent::ComboEnded { .. })));
        assert!(!c.is_busy());
    }

    #[test]
    fn no_link_ends_combo() {
        let mut c = AttackCombo::light_chain_3();
        c.input(ComboInput::Attack);
        c.tick(0.1);
        // Wait out recovery without input
        let mut ended = false;
        for _ in 0..20 {
            let e = c.tick(0.05);
            if e.iter()
                .any(|ev| matches!(ev, ComboEvent::ComboEnded { steps: 1 }))
            {
                ended = true;
                break;
            }
        }
        assert!(ended);
    }

    #[test]
    fn special_cancel() {
        let mut c = AttackCombo::new(vec![
            ComboStep::new("a", 1.0, 0.2, 0.2, 0.1).with_special_cancel(0.15)
        ]);
        c.input(ComboInput::Attack);
        c.tick(0.05);
        let e = c.input(ComboInput::Special);
        assert!(matches!(e[0], ComboEvent::SpecialCancel { from_index: 0 }));
        assert!(!c.is_busy());
    }

    #[test]
    fn damage_mul_active() {
        let mut c = AttackCombo::light_chain_3();
        assert!((c.damage_mul() - 1.0).abs() < 1e-5);
        c.input(ComboInput::Attack);
        assert!((c.damage_mul() - 1.0).abs() < 1e-5);
        assert!(c.is_active_frames());
    }
}
