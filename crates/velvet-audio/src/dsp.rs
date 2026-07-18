//! DSP helpers: ADSR envelopes, one-pole lowpass, ducking, voice priority.

use crate::voice::{Voice, VoiceState};

/// ADSR envelope stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AdsrStage {
    /// Idle / finished.
    #[default]
    Idle,
    /// Attack ramp 0 → 1.
    Attack,
    /// Decay ramp 1 → sustain.
    Decay,
    /// Sustain hold.
    Sustain,
    /// Release ramp → 0.
    Release,
}

/// ADSR envelope parameters (seconds for A/D/R, sustain level 0..=1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrParams {
    /// Attack time seconds.
    pub attack: f32,
    /// Decay time seconds.
    pub decay: f32,
    /// Sustain level 0..=1.
    pub sustain: f32,
    /// Release time seconds.
    pub release: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

impl AdsrParams {
    /// Percussive short envelope.
    pub fn percussive() -> Self {
        Self {
            attack: 0.001,
            decay: 0.15,
            sustain: 0.0,
            release: 0.05,
        }
    }

    /// Soft pad-like envelope.
    pub fn pad() -> Self {
        Self {
            attack: 0.4,
            decay: 0.3,
            sustain: 0.8,
            release: 0.8,
        }
    }
}

/// Runtime ADSR state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct AdsrEnvelope {
    /// Parameters.
    pub params: AdsrParams,
    stage: AdsrStage,
    /// Current level 0..=1.
    level: f32,
    /// Time in current stage.
    stage_time: f32,
    /// Level at start of release.
    release_from: f32,
    /// Note is held (gate on).
    gate: bool,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self::new(AdsrParams::default())
    }
}

impl AdsrEnvelope {
    /// Create idle envelope.
    pub fn new(params: AdsrParams) -> Self {
        Self {
            params,
            stage: AdsrStage::Idle,
            level: 0.0,
            stage_time: 0.0,
            release_from: 0.0,
            gate: false,
        }
    }

    /// Current stage.
    pub fn stage(&self) -> AdsrStage {
        self.stage
    }

    /// Current amplitude level.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Whether actively producing non-idle output.
    pub fn is_active(&self) -> bool {
        !matches!(self.stage, AdsrStage::Idle)
    }

    /// Note on — start attack.
    pub fn note_on(&mut self) {
        self.gate = true;
        self.stage = AdsrStage::Attack;
        self.stage_time = 0.0;
    }

    /// Note off — enter release if not idle.
    pub fn note_off(&mut self) {
        self.gate = false;
        if !matches!(self.stage, AdsrStage::Idle) {
            self.release_from = self.level;
            self.stage = AdsrStage::Release;
            self.stage_time = 0.0;
        }
    }

    /// Hard reset to idle.
    pub fn reset(&mut self) {
        self.stage = AdsrStage::Idle;
        self.level = 0.0;
        self.stage_time = 0.0;
        self.gate = false;
    }

    /// Advance by `dt` seconds; returns current level.
    pub fn tick(&mut self, dt: f32) -> f32 {
        let dt = dt.max(0.0);
        match self.stage {
            AdsrStage::Idle => {
                self.level = 0.0;
            }
            AdsrStage::Attack => {
                let a = self.params.attack.max(1e-6);
                self.stage_time += dt;
                self.level = (self.stage_time / a).min(1.0);
                if self.stage_time >= a {
                    self.stage = AdsrStage::Decay;
                    self.stage_time = 0.0;
                    self.level = 1.0;
                }
            }
            AdsrStage::Decay => {
                let d = self.params.decay.max(1e-6);
                let sustain = self.params.sustain.clamp(0.0, 1.0);
                self.stage_time += dt;
                let t = (self.stage_time / d).min(1.0);
                self.level = 1.0 + (sustain - 1.0) * t;
                if self.stage_time >= d {
                    self.level = sustain;
                    if sustain <= 1e-6 && !self.gate {
                        self.stage = AdsrStage::Idle;
                        self.level = 0.0;
                    } else if self.gate {
                        self.stage = AdsrStage::Sustain;
                        self.stage_time = 0.0;
                    } else {
                        self.release_from = self.level;
                        self.stage = AdsrStage::Release;
                        self.stage_time = 0.0;
                    }
                }
            }
            AdsrStage::Sustain => {
                self.level = self.params.sustain.clamp(0.0, 1.0);
                if !self.gate {
                    self.release_from = self.level;
                    self.stage = AdsrStage::Release;
                    self.stage_time = 0.0;
                }
            }
            AdsrStage::Release => {
                let r = self.params.release.max(1e-6);
                self.stage_time += dt;
                let t = (self.stage_time / r).min(1.0);
                self.level = self.release_from * (1.0 - t);
                if self.stage_time >= r {
                    self.level = 0.0;
                    self.stage = AdsrStage::Idle;
                }
            }
        }
        self.level
    }
}

/// One-pole low-pass filter state (per sample or control-rate).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LowPass1P {
    /// Smoothed value / last output.
    y: f32,
    /// Cutoff coefficient in (0, 1]; higher = brighter / less smoothing.
    alpha: f32,
}

impl Default for LowPass1P {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl LowPass1P {
    /// Create with alpha (0 exclusive..=1). Small alpha = heavier lowpass.
    pub fn new(alpha: f32) -> Self {
        Self {
            y: 0.0,
            alpha: alpha.clamp(1e-6, 1.0),
        }
    }

    /// Design from cutoff Hz and sample rate.
    pub fn from_cutoff(cutoff_hz: f32, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let fc = cutoff_hz.clamp(0.0, sr * 0.49);
        // Simple one-pole: alpha ≈ 1 - exp(-2π fc/sr)
        let alpha = 1.0 - (-std::f32::consts::TAU * fc / sr).exp();
        Self::new(alpha)
    }

    /// Set alpha.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha.clamp(1e-6, 1.0);
    }

    /// Current alpha.
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Process one sample.
    pub fn process(&mut self, x: f32) -> f32 {
        self.y += self.alpha * (x - self.y);
        self.y
    }

    /// Process a buffer in place.
    pub fn process_buffer(&mut self, samples: &mut [f32]) {
        for s in samples {
            *s = self.process(*s);
        }
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.y = 0.0;
    }

    /// Last output.
    pub fn value(&self) -> f32 {
        self.y
    }
}

/// Ducking bus: when sidechain is hot, reduce music bus gain.
#[derive(Debug, Clone, PartialEq)]
pub struct DuckingBus {
    /// Target gain when fully ducked (0..=1).
    pub duck_gain: f32,
    /// Attack time to duck in seconds.
    pub attack: f32,
    /// Release time to recover in seconds.
    pub release: f32,
    /// Sidechain threshold (linear).
    pub threshold: f32,
    /// Current gain multiplier applied to the ducked bus.
    gain: f32,
    /// Smoothed sidechain level.
    detector: LowPass1P,
}

impl Default for DuckingBus {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBus {
    /// Sensible defaults for voice-over-music ducking.
    pub fn new() -> Self {
        Self {
            duck_gain: 0.25,
            attack: 0.05,
            release: 0.4,
            threshold: 0.05,
            gain: 1.0,
            detector: LowPass1P::new(0.2),
        }
    }

    /// Current ducked bus gain.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Tick with sidechain absolute level (e.g. peak of voice bus) and dt.
    pub fn tick(&mut self, sidechain_level: f32, dt: f32) -> f32 {
        let level = self.detector.process(sidechain_level.abs());
        let target = if level >= self.threshold {
            self.duck_gain.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let dt = dt.max(0.0);
        let coeff = if target < self.gain {
            // ducking down — attack
            1.0 - (-dt / self.attack.max(1e-4)).exp()
        } else {
            1.0 - (-dt / self.release.max(1e-4)).exp()
        };
        self.gain += (target - self.gain) * coeff.clamp(0.0, 1.0);
        self.gain
    }

    /// Apply gain to a sample.
    pub fn apply(&self, sample: f32) -> f32 {
        sample * self.gain
    }
}

/// Voice prioritization: sort and optionally steal lowest priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoicePriorityKey {
    /// Higher priority survives.
    pub priority: i32,
    /// Older voices steal first when priority ties (higher age = steal first).
    pub age_frames: u32,
    /// Soft: fading-out preferred for steal.
    pub fading: bool,
    /// Index into voice list.
    pub index: usize,
}

impl PartialOrd for VoicePriorityKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VoicePriorityKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort ascending by keep-score so first is steal candidate.
        // Lower priority steals first; if equal, higher age; if equal, fading first.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.age_frames.cmp(&self.age_frames))
            .then_with(|| other.fading.cmp(&self.fading))
            .then_with(|| self.index.cmp(&other.index))
    }
}

/// Build sort keys for active voices (steal order: first = most stealable).
pub fn voice_steal_order(voices: &[Voice], ages: &[u32]) -> Vec<VoicePriorityKey> {
    let mut keys = Vec::new();
    for (i, v) in voices.iter().enumerate() {
        if !v.is_active() {
            continue;
        }
        let age = ages.get(i).copied().unwrap_or(0);
        keys.push(VoicePriorityKey {
            priority: v.priority,
            age_frames: age,
            fading: matches!(v.state, VoiceState::FadingOut),
            index: i,
        });
    }
    keys.sort();
    keys
}

/// Sort voices for mix order: higher priority first, then newer.
pub fn sort_voices_for_mix(voices: &mut [Voice], ages: &[u32]) {
    let mut order: Vec<usize> = (0..voices.len()).collect();
    order.sort_by(|&a, &b| {
        let pa = voices[a].priority;
        let pb = voices[b].priority;
        pb.cmp(&pa).then_with(|| {
            let aa = ages.get(a).copied().unwrap_or(0);
            let ab = ages.get(b).copied().unwrap_or(0);
            aa.cmp(&ab)
        })
    });
    // Apply permutation
    let clone = voices.to_vec();
    for (i, &src) in order.iter().enumerate() {
        voices[i] = clone[src].clone();
    }
}

/// Select indices to stop when over `max_voices`.
pub fn select_voices_to_steal(voices: &[Voice], ages: &[u32], max_voices: usize) -> Vec<usize> {
    let active: Vec<usize> = voices
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_active())
        .map(|(i, _)| i)
        .collect();
    if active.len() <= max_voices {
        return Vec::new();
    }
    let need = active.len() - max_voices;
    let order = voice_steal_order(voices, ages);
    order.into_iter().take(need).map(|k| k.index).collect()
}

/// Simple gain ramp helper for fades.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GainRamp {
    current: f32,
    target: f32,
    speed: f32,
}

impl GainRamp {
    /// Create at level.
    pub fn new(level: f32) -> Self {
        Self {
            current: level,
            target: level,
            speed: 1.0,
        }
    }

    /// Set target and speed (units per second).
    pub fn set_target(&mut self, target: f32, speed: f32) {
        self.target = target;
        self.speed = speed.max(0.0);
    }

    /// Tick toward target.
    pub fn tick(&mut self, dt: f32) -> f32 {
        let dt = dt.max(0.0);
        let diff = self.target - self.current;
        let step = self.speed * dt;
        if diff.abs() <= step {
            self.current = self.target;
        } else {
            self.current += diff.signum() * step;
        }
        self.current
    }

    /// Current gain.
    pub fn value(&self) -> f32 {
        self.current
    }

    /// Whether at target.
    pub fn is_settled(&self) -> bool {
        (self.current - self.target).abs() < 1e-5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::BusId;
    use crate::clip::ClipId;
    use crate::engine::PlaybackId;

    fn dummy_voice(priority: i32, state: VoiceState) -> Voice {
        Voice {
            id: PlaybackId::from_raw(1),
            clip: ClipId::allocate(),
            bus: BusId::from_kind(crate::bus::BusKind::Effects),
            state,
            volume: 1.0,
            pan: 0.0,
            cursor_secs: 0.0,
            duration_secs: 1.0,
            looping: false,
            priority,
            fade_to: None,
            fade_speed: 0.0,
            position: None,
        }
    }

    #[test]
    fn adsr_attack_sustain_release() {
        let mut env = AdsrEnvelope::new(AdsrParams {
            attack: 0.1,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        });
        env.note_on();
        let mut peak: f32 = 0.0;
        for _ in 0..20 {
            peak = peak.max(env.tick(0.02));
        }
        assert!(peak > 0.9);
        assert!(matches!(env.stage(), AdsrStage::Sustain | AdsrStage::Decay));
        env.note_off();
        for _ in 0..20 {
            env.tick(0.02);
        }
        assert!(!env.is_active());
        assert!(env.level() < 1e-3);
    }

    #[test]
    fn lowpass_smooths() {
        let mut lp = LowPass1P::new(0.1);
        let mut y = 0.0;
        for _ in 0..5 {
            y = lp.process(1.0);
        }
        assert!(y > 0.0 && y < 1.0);
        for _ in 0..100 {
            y = lp.process(1.0);
        }
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn ducking_reduces_gain() {
        let mut d = DuckingBus::new();
        d.threshold = 0.1;
        d.duck_gain = 0.2;
        // Hot sidechain
        for _ in 0..30 {
            d.tick(1.0, 0.016);
        }
        assert!(d.gain() < 0.5);
        // Silence
        for _ in 0..100 {
            d.tick(0.0, 0.016);
        }
        assert!(d.gain() > 0.9);
    }

    #[test]
    fn steal_lowest_priority() {
        let voices = vec![
            dummy_voice(10, VoiceState::Playing),
            dummy_voice(1, VoiceState::Playing),
            dummy_voice(5, VoiceState::FadingOut),
        ];
        let ages = [5, 1, 3];
        let steal = select_voices_to_steal(&voices, &ages, 2);
        assert_eq!(steal.len(), 1);
        assert_eq!(steal[0], 1); // priority 1
    }

    #[test]
    fn gain_ramp() {
        let mut g = GainRamp::new(0.0);
        g.set_target(1.0, 2.0);
        g.tick(0.25);
        assert!((g.value() - 0.5).abs() < 1e-4);
        g.tick(1.0);
        assert!(g.is_settled());
    }
}
