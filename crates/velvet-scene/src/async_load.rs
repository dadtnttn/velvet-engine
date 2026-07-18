//! Asynchronous scene load simulation via a deterministic state machine.
//!
//! This does not perform real OS async I/O; it models load phases so gameplay
//! can drive progress bars, cancellation, and staging without a runtime.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::manager::{SceneBlueprint, SceneManager, SceneManagerError};
use crate::prefab::PrefabLibrary;
use velvet_ecs::World;

/// Async load errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AsyncLoadError {
    /// Invalid state transition.
    #[error("invalid load state: {0}")]
    InvalidState(String),
    /// Cancelled.
    #[error("load cancelled")]
    Cancelled,
    /// Manager error.
    #[error(transparent)]
    Manager(#[from] SceneManagerError),
    /// Missing job.
    #[error("load job not found: {0}")]
    NotFound(String),
}

/// Phase of a simulated async load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadPhase {
    /// Queued, not started.
    Pending,
    /// Resolving asset dependencies.
    ResolvingDeps,
    /// Streaming asset bytes (simulated).
    Streaming,
    /// Instantiating entities into a staging world.
    Instantiating,
    /// Activating into the live world.
    Activating,
    /// Completed successfully.
    Done,
    /// Failed or cancelled.
    Failed,
}

impl LoadPhase {
    /// Progress fraction hint 0..=1 for UI.
    pub fn progress_hint(self) -> f32 {
        match self {
            Self::Pending => 0.0,
            Self::ResolvingDeps => 0.15,
            Self::Streaming => 0.45,
            Self::Instantiating => 0.75,
            Self::Activating => 0.9,
            Self::Done => 1.0,
            Self::Failed => 0.0,
        }
    }

    /// Terminal?
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed)
    }
}

/// One load job.
#[derive(Debug, Clone)]
pub struct LoadJob {
    /// Job id.
    pub id: String,
    /// Blueprint to load.
    pub blueprint: SceneBlueprint,
    /// Additive flag.
    pub additive: bool,
    /// Current phase.
    pub phase: LoadPhase,
    /// Simulated work units remaining in current phase.
    pub phase_work: u32,
    /// Error message if failed.
    pub error: Option<String>,
    /// Resulting scene id once done (stringified).
    pub result_scene: Option<String>,
}

impl LoadJob {
    /// Progress 0..=1 combining phase hint and remaining work.
    pub fn progress(&self) -> f32 {
        if self.phase.is_terminal() {
            return self.phase.progress_hint();
        }
        let base = self.phase.progress_hint();
        let next = match self.phase {
            LoadPhase::Pending => LoadPhase::ResolvingDeps,
            LoadPhase::ResolvingDeps => LoadPhase::Streaming,
            LoadPhase::Streaming => LoadPhase::Instantiating,
            LoadPhase::Instantiating => LoadPhase::Activating,
            LoadPhase::Activating => LoadPhase::Done,
            LoadPhase::Done | LoadPhase::Failed => self.phase,
        }
        .progress_hint();
        let t = if self.phase_work == 0 {
            1.0
        } else {
            // Rough: less work left → closer to next
            1.0 - (self.phase_work as f32 / (self.phase_work as f32 + 1.0))
        };
        base + (next - base) * t * 0.5
    }
}

/// Scheduler for simulated async loads.
#[derive(Debug, Default)]
pub struct AsyncSceneLoader {
    jobs: IndexMap<String, LoadJob>,
    next_id: u64,
    /// Work units granted per phase by default.
    pub default_phase_work: u32,
}

impl AsyncSceneLoader {
    /// Create with default phase work.
    pub fn new() -> Self {
        Self {
            default_phase_work: 3,
            ..Default::default()
        }
    }

    /// Queue a blueprint load; returns job id.
    pub fn enqueue(&mut self, blueprint: SceneBlueprint, additive: bool) -> String {
        self.next_id += 1;
        let id = format!("load_{}", self.next_id);
        let job = LoadJob {
            id: id.clone(),
            blueprint,
            additive,
            phase: LoadPhase::Pending,
            phase_work: self.default_phase_work,
            error: None,
            result_scene: None,
        };
        self.jobs.insert(id.clone(), job);
        id
    }

    /// Get job.
    pub fn job(&self, id: &str) -> Option<&LoadJob> {
        self.jobs.get(id)
    }

    /// Active (non-terminal) jobs.
    pub fn active_jobs(&self) -> impl Iterator<Item = &LoadJob> {
        self.jobs.values().filter(|j| !j.phase.is_terminal())
    }

    /// Cancel a job.
    pub fn cancel(&mut self, id: &str) -> Result<(), AsyncLoadError> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| AsyncLoadError::NotFound(id.into()))?;
        if job.phase.is_terminal() {
            return Err(AsyncLoadError::InvalidState(
                "cannot cancel terminal job".into(),
            ));
        }
        job.phase = LoadPhase::Failed;
        job.error = Some("cancelled".into());
        Ok(())
    }

    /// Advance all active jobs by one simulation step.
    /// When a job reaches Activating with work done, it calls into SceneManager::load.
    pub fn tick(
        &mut self,
        world: &mut World,
        manager: &mut SceneManager,
        library: &PrefabLibrary,
    ) -> Result<Vec<String>, AsyncLoadError> {
        let ids: Vec<String> = self
            .jobs
            .values()
            .filter(|j| !j.phase.is_terminal())
            .map(|j| j.id.clone())
            .collect();
        let mut completed = Vec::new();
        for id in ids {
            if self.advance_one(&id, world, manager, library)? {
                completed.push(id);
            }
        }
        Ok(completed)
    }

    fn advance_one(
        &mut self,
        id: &str,
        world: &mut World,
        manager: &mut SceneManager,
        library: &PrefabLibrary,
    ) -> Result<bool, AsyncLoadError> {
        let default_work = self.default_phase_work;
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| AsyncLoadError::NotFound(id.into()))?;

        if job.phase_work > 0 {
            job.phase_work -= 1;
            return Ok(false);
        }

        match job.phase {
            LoadPhase::Pending => {
                job.phase = LoadPhase::ResolvingDeps;
                job.phase_work = default_work;
            }
            LoadPhase::ResolvingDeps => {
                job.phase = LoadPhase::Streaming;
                job.phase_work = default_work;
            }
            LoadPhase::Streaming => {
                job.phase = LoadPhase::Instantiating;
                job.phase_work = default_work;
            }
            LoadPhase::Instantiating => {
                job.phase = LoadPhase::Activating;
                job.phase_work = 0;
            }
            LoadPhase::Activating => {
                let bp = job.blueprint.clone();
                let additive = job.additive;
                match manager.load(world, library, &bp, additive) {
                    Ok(scene_id) => {
                        job.phase = LoadPhase::Done;
                        job.result_scene = Some(format!("{scene_id:?}"));
                        return Ok(true);
                    }
                    Err(e) => {
                        job.phase = LoadPhase::Failed;
                        job.error = Some(e.to_string());
                        return Err(e.into());
                    }
                }
            }
            LoadPhase::Done | LoadPhase::Failed => {}
        }
        Ok(false)
    }

    /// Force-complete pending work for tests (still phases through).
    pub fn flush(
        &mut self,
        world: &mut World,
        manager: &mut SceneManager,
        library: &PrefabLibrary,
    ) -> Result<Vec<String>, AsyncLoadError> {
        let mut done = Vec::new();
        for _ in 0..64 {
            let finished = self.tick(world, manager, library)?;
            done.extend(finished);
            if self.active_jobs().next().is_none() {
                break;
            }
        }
        Ok(done)
    }

    /// Remove terminal jobs.
    pub fn gc(&mut self) {
        self.jobs.retain(|_, j| !j.phase.is_terminal());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefab::{Prefab, PrefabLibrary};
    use velvet_math::Vec2;

    #[test]
    fn load_reaches_done() {
        let mut loader = AsyncSceneLoader::new();
        loader.default_phase_work = 1;
        let mut library = PrefabLibrary::default();
        library.insert(Prefab::simple("p", "ent", Vec2::ZERO));
        let bp = SceneBlueprint::new("level1").with_entity("e1", "p");
        let id = loader.enqueue(bp, false);
        let mut world = World::new();
        let mut manager = SceneManager::new();
        let completed = loader.flush(&mut world, &mut manager, &library).unwrap();
        assert!(completed.contains(&id));
        assert_eq!(loader.job(&id).unwrap().phase, LoadPhase::Done);
        assert!(world.entity_count() >= 1);
    }

    #[test]
    fn cancel_midway() {
        let mut loader = AsyncSceneLoader::new();
        loader.default_phase_work = 5;
        let bp = SceneBlueprint::new("x");
        let id = loader.enqueue(bp, true);
        loader.cancel(&id).unwrap();
        assert_eq!(loader.job(&id).unwrap().phase, LoadPhase::Failed);
    }

    #[test]
    fn progress_increases() {
        let mut loader = AsyncSceneLoader::new();
        loader.default_phase_work = 2;
        let id = loader.enqueue(SceneBlueprint::new("a"), false);
        let p0 = loader.job(&id).unwrap().progress();
        let mut world = World::new();
        let mut manager = SceneManager::new();
        let library = PrefabLibrary::default();
        let _ = loader.tick(&mut world, &mut manager, &library);
        let p1 = loader.job(&id).unwrap().progress();
        assert!(p1 >= p0);
    }
}
