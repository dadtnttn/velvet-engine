//! Wave spawner state machine for action arenas.

use serde::{Deserialize, Serialize};

/// One enemy spawn request produced by the arena.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Wave index (0-based).
    pub wave: u32,
    /// Enemy archetype / kind id.
    pub kind: String,
    /// Spawn slot index within the wave.
    pub slot: usize,
    /// Optional spawn point id.
    #[serde(default)]
    pub spawn_point: Option<String>,
}

/// Definition of a single wave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveDef {
    /// Entries: (kind, count).
    pub enemies: Vec<(String, u32)>,
    /// Delay before the wave starts after the previous clears (seconds).
    pub intro_delay: f32,
    /// Optional spawn point ids cycled for this wave.
    #[serde(default)]
    pub spawn_points: Vec<String>,
}

impl WaveDef {
    /// Create a wave.
    pub fn new(enemies: Vec<(String, u32)>) -> Self {
        Self {
            enemies,
            intro_delay: 1.0,
            spawn_points: Vec::new(),
        }
    }

    /// Builder: intro delay.
    pub fn with_delay(mut self, secs: f32) -> Self {
        self.intro_delay = secs.max(0.0);
        self
    }

    /// Builder: spawn points.
    pub fn with_spawn_points(mut self, points: Vec<String>) -> Self {
        self.spawn_points = points;
        self
    }

    /// Total enemies in the wave.
    pub fn total_enemies(&self) -> u32 {
        self.enemies.iter().map(|(_, c)| *c).sum()
    }
}

/// High-level arena phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArenaPhase {
    /// Waiting to start (before first wave).
    #[default]
    Ready,
    /// Counting down intro delay.
    WaveIntro,
    /// Spawning / combat; wait until alive enemies reach 0.
    Combat,
    /// All waves cleared.
    Victory,
    /// Player failed (host sets this).
    Defeat,
}

/// Wave spawner / arena controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaController {
    /// Wave definitions.
    pub waves: Vec<WaveDef>,
    /// Current wave index.
    pub wave_index: u32,
    /// Phase.
    pub phase: ArenaPhase,
    /// Timer for intro delay.
    pub timer: f32,
    /// Enemies still alive for the current wave.
    pub alive: u32,
    /// Whether the current wave's spawn requests have been emitted.
    pub spawned: bool,
    /// Total kills this arena run.
    pub kills: u32,
    /// Loop waves after victory (endless mode).
    pub endless: bool,
}

impl Default for ArenaController {
    fn default() -> Self {
        Self {
            waves: Vec::new(),
            wave_index: 0,
            phase: ArenaPhase::Ready,
            timer: 0.0,
            alive: 0,
            spawned: false,
            kills: 0,
            endless: false,
        }
    }
}

impl ArenaController {
    /// Create with waves.
    pub fn new(waves: Vec<WaveDef>) -> Self {
        Self {
            waves,
            ..Default::default()
        }
    }

    /// Simple demo arena: 3 escalating waves.
    pub fn demo() -> Self {
        Self::new(vec![
            WaveDef::new(vec![("slime".into(), 3)]).with_delay(0.5),
            WaveDef::new(vec![("slime".into(), 2), ("bat".into(), 2)]).with_delay(1.0),
            WaveDef::new(vec![("brute".into(), 1), ("bat".into(), 3)]).with_delay(1.5),
        ])
    }

    /// Number of waves.
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }

    /// Start the arena (begins first wave intro).
    pub fn start(&mut self) {
        if self.waves.is_empty() {
            self.phase = ArenaPhase::Victory;
            return;
        }
        self.wave_index = 0;
        self.kills = 0;
        self.begin_wave_intro();
    }

    fn begin_wave_intro(&mut self) {
        let delay = self
            .waves
            .get(self.wave_index as usize)
            .map(|w| w.intro_delay)
            .unwrap_or(0.0);
        self.phase = ArenaPhase::WaveIntro;
        self.timer = delay;
        self.alive = 0;
        self.spawned = false;
    }

    /// Current wave def.
    pub fn current_wave(&self) -> Option<&WaveDef> {
        self.waves.get(self.wave_index as usize)
    }

    /// Build spawn requests for the current wave (call once when entering combat).
    pub fn take_spawn_requests(&mut self) -> Vec<SpawnRequest> {
        if self.spawned || self.phase != ArenaPhase::Combat {
            return Vec::new();
        }
        let Some(wave) = self.waves.get(self.wave_index as usize) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut slot = 0usize;
        for (kind, count) in &wave.enemies {
            for _ in 0..*count {
                let spawn_point = if wave.spawn_points.is_empty() {
                    None
                } else {
                    Some(wave.spawn_points[slot % wave.spawn_points.len()].clone())
                };
                out.push(SpawnRequest {
                    wave: self.wave_index,
                    kind: kind.clone(),
                    slot,
                    spawn_point,
                });
                slot += 1;
            }
        }
        self.alive = wave.total_enemies();
        self.spawned = true;
        out
    }

    /// Notify that an enemy died.
    pub fn on_enemy_killed(&mut self) {
        if self.phase != ArenaPhase::Combat {
            return;
        }
        self.kills += 1;
        self.alive = self.alive.saturating_sub(1);
        if self.alive == 0 {
            self.advance_after_clear();
        }
    }

    /// Set alive count explicitly (if spawns fail, etc.).
    pub fn set_alive(&mut self, alive: u32) {
        self.alive = alive;
        if self.phase == ArenaPhase::Combat && self.alive == 0 && self.spawned {
            self.advance_after_clear();
        }
    }

    fn advance_after_clear(&mut self) {
        let next = self.wave_index + 1;
        if next as usize >= self.waves.len() {
            if self.endless {
                self.wave_index = 0;
                self.begin_wave_intro();
            } else {
                self.phase = ArenaPhase::Victory;
            }
        } else {
            self.wave_index = next;
            self.begin_wave_intro();
        }
    }

    /// Host reports player death.
    pub fn on_player_defeat(&mut self) {
        self.phase = ArenaPhase::Defeat;
    }

    /// Tick intro timers. When combat begins, returns true once (caller should spawn).
    pub fn tick(&mut self, dt: f32) -> bool {
        let dt = dt.max(0.0);
        if self.phase == ArenaPhase::WaveIntro {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.phase = ArenaPhase::Combat;
                self.timer = 0.0;
                return true;
            }
        }
        false
    }

    /// Whether victory.
    pub fn is_victory(&self) -> bool {
        self.phase == ArenaPhase::Victory
    }

    /// Whether defeat.
    pub fn is_defeat(&self) -> bool {
        self.phase == ArenaPhase::Defeat
    }

    /// Progress text helper: "Wave 2/3".
    pub fn wave_label(&self) -> String {
        let total = self.waves.len().max(1);
        let cur = (self.wave_index as usize + 1).min(total);
        format!("Wave {cur}/{total}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_all_waves() {
        let mut arena = ArenaController::demo();
        arena.start();
        assert_eq!(arena.phase, ArenaPhase::WaveIntro);
        assert!(arena.tick(1.0)); // enter combat wave 0
        let spawns = arena.take_spawn_requests();
        assert_eq!(spawns.len(), 3);
        assert!(arena.take_spawn_requests().is_empty()); // once
        for _ in 0..3 {
            arena.on_enemy_killed();
        }
        assert_eq!(arena.phase, ArenaPhase::WaveIntro);
        assert_eq!(arena.wave_index, 1);
        // clear remaining waves
        while !arena.is_victory() {
            if arena.phase == ArenaPhase::WaveIntro {
                arena.tick(10.0);
            }
            if arena.phase == ArenaPhase::Combat {
                let n = arena.take_spawn_requests().len() as u32;
                if n == 0 {
                    // already spawned
                    let alive = arena.alive;
                    for _ in 0..alive {
                        arena.on_enemy_killed();
                    }
                } else {
                    for _ in 0..n {
                        arena.on_enemy_killed();
                    }
                }
            }
            if arena.phase == ArenaPhase::Ready {
                break;
            }
        }
        assert!(arena.is_victory());
        assert_eq!(arena.kills, 3 + 4 + 4);
    }

    #[test]
    fn defeat() {
        let mut arena = ArenaController::demo();
        arena.start();
        arena.on_player_defeat();
        assert!(arena.is_defeat());
    }

    #[test]
    fn endless_loops() {
        let mut arena =
            ArenaController::new(vec![WaveDef::new(vec![("a".into(), 1)]).with_delay(0.0)]);
        arena.endless = true;
        arena.start();
        arena.tick(0.0);
        let _ = arena.take_spawn_requests();
        arena.on_enemy_killed();
        assert_eq!(arena.phase, ArenaPhase::WaveIntro);
        assert_eq!(arena.wave_index, 0);
    }

    #[test]
    fn spawn_points_cycle() {
        let mut arena = ArenaController::new(vec![WaveDef::new(vec![("x".into(), 3)])
            .with_delay(0.0)
            .with_spawn_points(vec!["p0".into(), "p1".into()])]);
        arena.start();
        arena.tick(0.0);
        let req = arena.take_spawn_requests();
        assert_eq!(req[0].spawn_point.as_deref(), Some("p0"));
        assert_eq!(req[1].spawn_point.as_deref(), Some("p1"));
        assert_eq!(req[2].spawn_point.as_deref(), Some("p0"));
    }

    #[test]
    fn empty_arena_victory() {
        let mut arena = ArenaController::new(vec![]);
        arena.start();
        assert!(arena.is_victory());
    }

    #[test]
    fn wave_label_and_progress() {
        let mut arena = ArenaController::demo();
        arena.start();
        let label0 = arena.wave_label();
        assert!(
            label0.contains('1') || label0.contains("Wave"),
            "label={label0}"
        );
        arena.tick(10.0);
        let _ = arena.take_spawn_requests();
        // Kill wave 0
        let alive = arena.alive;
        for _ in 0..alive {
            arena.on_enemy_killed();
        }
        let label1 = arena.wave_label();
        assert_ne!(label0, label1);
        assert!(arena.kills > 0);
    }

    #[test]
    fn defeat_during_combat_stops() {
        let mut arena = ArenaController::demo();
        arena.start();
        arena.tick(1.0);
        assert_eq!(arena.phase, ArenaPhase::Combat);
        arena.on_player_defeat();
        assert!(arena.is_defeat());
        // Further ticks should not revive.
        arena.tick(1.0);
        assert!(arena.is_defeat());
    }

    #[test]
    fn custom_single_wave_victory() {
        let mut arena = ArenaController::new(vec![WaveDef::new(vec![
            ("slime".into(), 2),
            ("bat".into(), 1),
        ])
        .with_delay(0.0)]);
        arena.start();
        arena.tick(0.0);
        let spawns = arena.take_spawn_requests();
        assert_eq!(spawns.len(), 3);
        for _ in 0..3 {
            arena.on_enemy_killed();
        }
        assert!(arena.is_victory());
        assert_eq!(arena.kills, 3);
    }

    #[test]
    fn endless_increases_kill_count_over_loops() {
        let mut arena =
            ArenaController::new(vec![WaveDef::new(vec![("x".into(), 1)]).with_delay(0.0)]);
        arena.endless = true;
        arena.start();
        for _ in 0..5 {
            if arena.phase == ArenaPhase::WaveIntro {
                arena.tick(0.0);
            }
            let n = arena.take_spawn_requests().len().max(arena.alive as usize);
            for _ in 0..n.max(1) {
                if arena.alive > 0 {
                    arena.on_enemy_killed();
                } else {
                    break;
                }
            }
        }
        assert!(arena.kills >= 3, "kills={}", arena.kills);
        assert!(!arena.is_defeat());
    }
}
