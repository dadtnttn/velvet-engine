//! Live2D-compatible model attach/show on the product presentation path.
//!
//! Ships a testable surface without requiring the proprietary Cubism SDK:
//! model descriptors (model3.json / moc3-compatible paths + expressions) attach
//! to [`PresentationState`] and show as layered sprites.

use serde::{Deserialize, Serialize};

use crate::product::{LayeredSprite, PresentationState};

/// Live2D (or Live2D-compatible) model descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2dModel {
    /// Stable character/model id.
    pub id: String,
    /// Path to model3.json / moc3 package root (relative to project).
    pub model_path: String,
    /// Optional texture atlas path.
    pub texture: Option<String>,
    /// Active expression name.
    pub expression: String,
    /// Motion / idle name.
    pub motion: String,
    /// Placement tag: left | center | right.
    pub at: String,
    /// Z-order.
    pub z: i32,
    /// Visible.
    pub visible: bool,
    /// Live parameters.
    #[serde(default)]
    pub params: Vec<Live2dParam>,
    /// Active motion (None = idle pose).
    #[serde(default)]
    pub active_motion: Option<ActiveLive2dMotion>,
    /// Hit areas in 0..=1 model space.
    #[serde(default)]
    pub hit_areas: Vec<Live2dHitArea>,
    /// Seconds until next blink.
    #[serde(default)]
    pub blink_in: f32,
    /// Blink progress 0..=1 while blinking.
    #[serde(default)]
    pub blink_t: f32,
    /// Breath phase radians.
    #[serde(default)]
    pub breath_phase: f32,
    /// Smoothed mouth openness (lip-sync).
    #[serde(default)]
    pub mouth: f32,
}

impl Live2dModel {
    /// Create a model attached at center stage.
    pub fn new(id: impl Into<String>, model_path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model_path: model_path.into(),
            texture: None,
            expression: "default".into(),
            motion: "idle".into(),
            at: "center".into(),
            z: 25,
            visible: false,
            params: vec![
                Live2dParam::new(params::EYE_L_OPEN, 1.0, 0.0, 1.0),
                Live2dParam::new(params::EYE_R_OPEN, 1.0, 0.0, 1.0),
                Live2dParam::new(params::MOUTH_OPEN_Y, 0.0, 0.0, 1.0),
                Live2dParam::new(params::BREATH, 0.0, -1.0, 1.0),
            ],
            active_motion: None,
            hit_areas: Vec::new(),
            blink_in: 2.5,
            blink_t: 1.0,
            breath_phase: 0.0,
            mouth: 0.0,
        }
    }

    /// Apply an authored definition (params, hit areas; keeps placement).
    pub fn apply_def(&mut self, def: &Live2dModelDef) {
        for p in &def.params {
            match self.params.iter_mut().find(|q| q.id == p.id) {
                Some(slot) => {
                    slot.min = p.min;
                    slot.max = p.max;
                    slot.set(p.value);
                }
                None => self.params.push(p.clone()),
            }
        }
        self.hit_areas = def.hit_areas.clone();
        if !def.expressions.contains(&self.expression) && !def.expressions.is_empty() {
            self.expression = def.expressions[0].clone();
        }
    }

    /// Get a parameter value.
    pub fn param(&self, id: &str) -> Option<f32> {
        self.params.iter().find(|p| p.id == id).map(|p| p.value)
    }

    /// Set a parameter (adds it clamped 0..=1 when unknown).
    pub fn set_param(&mut self, id: &str, value: f32) {
        match self.params.iter_mut().find(|p| p.id == id) {
            Some(p) => p.set(value),
            None => self.params.push(Live2dParam::new(id, value, 0.0, 1.0)),
        }
    }

    /// Hit test in 0..=1 model space. Returns area id.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        self.hit_areas
            .iter()
            .find(|a| a.contains(x, y))
            .map(|a| a.id.as_str())
    }
}

/// A motion currently playing on a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveLive2dMotion {
    /// Motion being played.
    pub motion: Live2dMotion,
    /// Elapsed seconds.
    pub elapsed: f32,
}

impl ActiveLive2dMotion {
    /// Finished?
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.motion.duration_secs
    }

    /// Fade envelope 0..=1 (in → sustain → out).
    pub fn envelope(&self) -> f32 {
        let t = self.elapsed;
        let d = self.motion.duration_secs.max(1e-4);
        let fade_in = if self.motion.fade_in <= 0.0 {
            1.0
        } else {
            (t / self.motion.fade_in).clamp(0.0, 1.0)
        };
        let remain = d - t;
        let fade_out = if self.motion.fade_out <= 0.0 {
            1.0
        } else {
            (remain / self.motion.fade_out).clamp(0.0, 1.0)
        };
        fade_in.min(fade_out).clamp(0.0, 1.0)
    }
}

/// Conventional parameter ids (Cubism-compatible names; hosts may add more).
pub mod params {
    /// Left eye openness 0..=1.
    pub const EYE_L_OPEN: &str = "ParamEyeLOpen";
    /// Right eye openness 0..=1.
    pub const EYE_R_OPEN: &str = "ParamEyeROpen";
    /// Mouth openness 0..=1 (lip-sync target).
    pub const MOUTH_OPEN_Y: &str = "ParamMouthOpenY";
    /// Breathing phase driver -1..=1.
    pub const BREATH: &str = "ParamBreath";
    /// Body X angle -30..=30.
    pub const ANGLE_X: &str = "ParamAngleX";
}

/// Motion priority: idle motions never preempt.
pub const MOTION_PRIORITY_IDLE: u8 = 0;
/// Motion priority: normal story motions.
pub const MOTION_PRIORITY_NORMAL: u8 = 1;
/// Motion priority: always preempts (cutscene hits, surprises).
pub const MOTION_PRIORITY_FORCE: u8 = 2;

/// One clampable model parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2dParam {
    /// Parameter id (`ParamEyeLOpen`, …).
    pub id: String,
    /// Current value.
    pub value: f32,
    /// Minimum.
    pub min: f32,
    /// Maximum.
    pub max: f32,
}

impl Live2dParam {
    /// New parameter clamped into range.
    pub fn new(id: impl Into<String>, value: f32, min: f32, max: f32) -> Self {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        Self {
            id: id.into(),
            value: value.clamp(min, max),
            min,
            max,
        }
    }

    /// Set clamped.
    pub fn set(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }
}

/// A motion clip (timed parameter performance, not pixel data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2dMotion {
    /// Motion group (`idle`, `tap`, `surprise`, …).
    pub group: String,
    /// Motion number inside the group.
    pub no: u32,
    /// Priority (`MOTION_PRIORITY_*`).
    pub priority: u8,
    /// Fade-in seconds.
    pub fade_in: f32,
    /// Fade-out seconds.
    pub fade_out: f32,
    /// Total duration seconds.
    pub duration_secs: f32,
}

impl Live2dMotion {
    /// New motion.
    pub fn new(
        group: impl Into<String>,
        no: u32,
        priority: u8,
        fade_in: f32,
        fade_out: f32,
        duration_secs: f32,
    ) -> Self {
        Self {
            group: group.into(),
            no,
            priority,
            fade_in: fade_in.max(0.0),
            fade_out: fade_out.max(0.0),
            duration_secs: duration_secs.max(0.1),
        }
    }
}

/// Clickable / hoverable rectangle on a model, in 0..=1 model space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2dHitArea {
    /// Area id (`head`, `body`, …).
    pub id: String,
    /// Left, top, width, height in 0..=1 model space.
    pub rect: [f32; 4],
}

impl Live2dHitArea {
    /// Hit test in 0..=1 model space.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect[0]
            && x <= self.rect[0] + self.rect[2]
            && y >= self.rect[1]
            && y <= self.rect[1] + self.rect[3]
    }
}

/// Authored model definition (our JSON format; Cubism `model3.json` paths
/// stay valid as `model_path` for external renderers).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Live2dModelDef {
    /// Default parameters.
    #[serde(default)]
    pub params: Vec<Live2dParam>,
    /// Available motions.
    #[serde(default)]
    pub motions: Vec<Live2dMotion>,
    /// Known expression names.
    #[serde(default)]
    pub expressions: Vec<String>,
    /// Hit areas.
    #[serde(default)]
    pub hit_areas: Vec<Live2dHitArea>,
}

impl Live2dModelDef {
    /// Parse from JSON text.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("live2d model def: {e}"))
    }

    /// Load from path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("io {}: {e}", path.display()))?;
        Self::from_json(&text)
    }
}

/// Push side for real renderers (Cubism SDK, custom deform rigs).
///
/// The stage computes parameters every [`Live2dStage::tick`]; a backend
/// polls [`Live2dStage::params_for`] and draws. [`NullBackend`] drops them.
pub trait Live2dBackend {
    /// Push one model's computed parameters.
    fn push_params(&mut self, model_id: &str, params: &[(String, f32)]);
}

/// Backend that drops parameters (headless / tests).
#[derive(Debug, Default)]
pub struct NullBackend {
    /// Frames received (for tests).
    pub frames: u64,
}

impl Live2dBackend for NullBackend {
    fn push_params(&mut self, _model_id: &str, _params: &[(String, f32)]) {
        self.frames += 1;
    }
}

/// Registry of Live2D models on the product presentation path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Live2dStage {
    /// Models by id.
    pub models: Vec<Live2dModel>,
}

impl Live2dStage {
    /// Attach (or replace) a model; does not show until [`Self::show`].
    pub fn attach(&mut self, model: Live2dModel) {
        if let Some(slot) = self.models.iter_mut().find(|m| m.id == model.id) {
            *slot = model;
        } else {
            self.models.push(model);
        }
    }

    /// Show model by id (marks visible). Returns false if missing.
    pub fn show(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.visible = true;
            true
        } else {
            false
        }
    }

    /// Hide model.
    pub fn hide(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.visible = false;
            true
        } else {
            false
        }
    }

    /// Set expression.
    pub fn set_expression(&mut self, id: &str, expression: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.expression = expression.into();
            true
        } else {
            false
        }
    }

    /// Play a motion. Lower priority than the active one is rejected
    /// (returns false); equal or higher preempts. Unknown model → false.
    pub fn play_motion(&mut self, id: &str, motion: Live2dMotion) -> bool {
        let Some(m) = self.models.iter_mut().find(|m| m.id == id) else {
            return false;
        };
        if let Some(active) = &m.active_motion {
            if !active.is_finished() && active.motion.priority > motion.priority {
                return false;
            }
        }
        m.motion = motion.group.clone();
        m.active_motion = Some(ActiveLive2dMotion {
            motion,
            elapsed: 0.0,
        });
        true
    }

    /// Stop the active motion (back to idle pose).
    pub fn stop_motion(&mut self, id: &str) -> bool {
        match self.models.iter_mut().find(|m| m.id == id) {
            Some(m) => {
                m.active_motion = None;
                m.motion = "idle".into();
                true
            }
            None => false,
        }
    }

    /// Tick blink, breath, motions and lip-sync.
    ///
    /// `voice_playing` drives procedural lip-flap while a voice line plays
    /// (no audio analysis; backends replace it with real levels).
    /// Finished one-shot motions clear back to idle automatically.
    pub fn tick(&mut self, dt: f32, voice_playing: bool) {
        let dt = dt.max(0.0);
        for m in self.models.iter_mut().filter(|m| m.visible) {
            // breath: slow sine on ParamBreath
            m.breath_phase += dt * 1.4;
            m.set_param(params::BREATH, m.breath_phase.sin());
            // blink: countdown then 0.12s close
            if m.blink_t >= 1.0 {
                m.blink_in -= dt;
                if m.blink_in <= 0.0 {
                    m.blink_in = 2.0 + (m.breath_phase.sin() * 0.5 + 0.5) * 2.5;
                    m.blink_t = 0.0;
                }
                m.set_param(params::EYE_L_OPEN, 1.0);
                m.set_param(params::EYE_R_OPEN, 1.0);
            } else {
                m.blink_t = (m.blink_t + dt / 0.12).min(1.0);
                // dip: 1 -> 0.1 -> 1 across the blink
                let dip = 1.0 - (m.blink_t * std::f32::consts::PI).sin() * 0.9;
                m.set_param(params::EYE_L_OPEN, dip);
                m.set_param(params::EYE_R_OPEN, dip);
            }
            // lip-sync flap while voice plays, smooth decay otherwise
            let target = if voice_playing {
                0.35 + 0.65 * (m.breath_phase * 7.0).sin().abs()
            } else {
                0.0
            };
            m.mouth += (target - m.mouth) * (1.0 - (-dt * 14.0).exp());
            m.set_param(params::MOUTH_OPEN_Y, m.mouth);
            // motion lifetime
            if let Some(active) = m.active_motion.as_mut() {
                active.elapsed += dt;
                if active.is_finished() {
                    m.active_motion = None;
                    m.motion = "idle".into();
                }
            }
        }
    }

    /// Computed `(param, value)` snapshot for a model (backend push).
    pub fn params_for(&self, id: &str) -> Option<Vec<(String, f32)>> {
        self.models
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.params.iter().map(|p| (p.id.clone(), p.value)).collect())
    }

    /// Push all visible models into a backend. Returns models pushed.
    pub fn push_to_backend(&self, backend: &mut impl Live2dBackend) -> usize {
        let mut n = 0;
        for m in self.models.iter().filter(|m| m.visible) {
            let params: Vec<(String, f32)> =
                m.params.iter().map(|p| (p.id.clone(), p.value)).collect();
            backend.push_params(&m.id, &params);
            n += 1;
        }
        n
    }

    /// Visible models.
    pub fn visible_models(&self) -> Vec<&Live2dModel> {
        self.models.iter().filter(|m| m.visible).collect()
    }

    /// Mirror visible Live2D models into presentation sprites (product show path).
    pub fn sync_presentation(&self, presentation: &mut PresentationState) {
        for m in self.models.iter().filter(|m| m.visible) {
            let sprite_id = format!("live2d:{}", m.id);
            presentation.sprites.insert(
                sprite_id.clone(),
                LayeredSprite {
                    id: sprite_id,
                    expression: Some(format!("{}@{}", m.expression, m.model_path)),
                    at: Some(m.at.clone()),
                    z: m.z,
                },
            );
        }
        let keep: std::collections::HashSet<String> = self
            .models
            .iter()
            .filter(|m| m.visible)
            .map(|m| format!("live2d:{}", m.id))
            .collect();
        presentation
            .sprites
            .retain(|k, _| !k.starts_with("live2d:") || keep.contains(k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_priority_preempts() {
        let mut stage = Live2dStage::default();
        stage.attach(Live2dModel::new("a", "m"));
        stage.show("a");
        assert!(stage.play_motion(
            "a",
            Live2dMotion::new("idle", 0, MOTION_PRIORITY_IDLE, 0.2, 0.2, 5.0)
        ));
        // normal preempts idle
        assert!(stage.play_motion(
            "a",
            Live2dMotion::new("tap", 1, MOTION_PRIORITY_NORMAL, 0.1, 0.1, 0.8)
        ));
        // idle cannot preempt normal
        assert!(!stage.play_motion(
            "a",
            Live2dMotion::new("idle", 0, MOTION_PRIORITY_IDLE, 0.2, 0.2, 5.0)
        ));
        // force always preempts
        assert!(stage.play_motion(
            "a",
            Live2dMotion::new("surprise", 0, MOTION_PRIORITY_FORCE, 0.05, 0.2, 0.5)
        ));
        assert!(!stage.play_motion(
            "missing",
            Live2dMotion::new("tap", 0, MOTION_PRIORITY_FORCE, 0.1, 0.1, 0.5)
        ));
        // finishes back to idle
        stage.tick(2.0, false);
        let m = &stage.models[0];
        assert!(m.active_motion.is_none());
        assert_eq!(m.motion, "idle");
    }

    #[test]
    fn blink_breath_and_lipsync() {
        let mut stage = Live2dStage::default();
        stage.attach(Live2dModel::new("a", "m"));
        stage.show("a");
        // breath oscillates
        stage.tick(0.5, false);
        let b1 = stage.models[0].param(params::BREATH).unwrap();
        stage.tick(0.5, false);
        let b2 = stage.models[0].param(params::BREATH).unwrap();
        assert!((b1 - b2).abs() > 1e-4, "breath should move: {b1} vs {b2}");
        // force a blink dip (first tick triggers, second dips mid-blink)
        stage.models[0].blink_in = 0.0;
        stage.models[0].blink_t = 1.0;
        stage.tick(0.06, false);
        stage.tick(0.06, false);
        let eye = stage.models[0].param(params::EYE_L_OPEN).unwrap();
        assert!(eye < 1.0, "eyes should dip mid-blink, got {eye}");
        stage.tick(1.0, false);
        assert!((stage.models[0].param(params::EYE_L_OPEN).unwrap() - 1.0).abs() < 1e-5);
        // lip-sync flaps only while voice plays
        stage.tick(0.2, true);
        assert!(stage.models[0].mouth > 0.1);
        stage.tick(2.0, false);
        assert!(stage.models[0].mouth < 0.05);
        // hidden models are skipped
        stage.hide("a");
        let mut backend = NullBackend::default();
        assert_eq!(stage.push_to_backend(&mut backend), 0);
        stage.show("a");
        assert_eq!(stage.push_to_backend(&mut backend), 1);
        assert_eq!(backend.frames, 1);
    }

    #[test]
    fn params_clamp_and_hit_areas() {
        let mut m = Live2dModel::new("a", "m");
        m.set_param("custom", 5.0);
        assert_eq!(m.param("custom"), Some(1.0));
        m.set_param(params::ANGLE_X, -99.0);
        assert_eq!(m.param(params::ANGLE_X), Some(0.0));
        m.hit_areas.push(Live2dHitArea {
            id: "head".into(),
            rect: [0.3, 0.0, 0.4, 0.3],
        });
        assert_eq!(m.hit_test(0.5, 0.1), Some("head"));
        assert_eq!(m.hit_test(0.1, 0.9), None);
    }

    #[test]
    fn model_def_applies() {
        let def = Live2dModelDef::from_json(
            r#"{"params": [{"id": "ParamAngleX", "value": 5.0, "min": -30.0, "max": 30.0}],
                "motions": [{"group": "tap", "no": 0, "priority": 1, "fade_in": 0.1, "fade_out": 0.2, "duration_secs": 0.9}],
                "expressions": ["smile"],
                "hit_areas": [{"id": "body", "rect": [0.0, 0.3, 1.0, 0.7]}]}"#,
        )
        .unwrap();
        let mut m = Live2dModel::new("a", "m");
        m.apply_def(&def);
        assert_eq!(m.param(params::ANGLE_X), Some(5.0));
        assert_eq!(m.expression, "smile");
        assert_eq!(m.hit_test(0.5, 0.5), Some("body"));
        assert!(Live2dModelDef::from_json("nope").is_err());
        let env = ActiveLive2dMotion {
            motion: def.motions[0].clone(),
            elapsed: 0.05,
        };
        assert!((env.envelope() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn attach_show_syncs_presentation() {
        let mut stage = Live2dStage::default();
        let mut model = Live2dModel::new("hero", "models/hero");
        model.texture = Some("models/hero/tex.png".into());
        stage.attach(model);
        assert!(!stage.show("missing"));
        assert!(stage.show("hero"));
        assert_eq!(stage.visible_models().len(), 1);
        stage.set_expression("hero", "smile");
        assert_eq!(stage.models[0].expression, "smile");

        let mut pres = PresentationState::default();
        stage.sync_presentation(&mut pres);
        assert!(pres.sprites.contains_key("live2d:hero"));
        let sp = &pres.sprites["live2d:hero"];
        assert!(sp
            .expression
            .as_ref()
            .map(|e| e.contains("smile"))
            .unwrap_or(false));
        stage.hide("hero");
        stage.sync_presentation(&mut pres);
        assert!(!pres.sprites.contains_key("live2d:hero"));
    }
}
