//! Enemy AI wrappers on play state machines.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;
use velvet_play::{BehaviorState, StateMachine};

/// Enemy archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyKind {
    /// Patrol only.
    Guard,
    /// Aggressive chase.
    Hunter,
}

/// Patrol waypoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatrolPath {
    /// Points.
    pub points: Vec<Vec2>,
    /// Current index.
    pub index: usize,
}

impl PatrolPath {
    /// Create.
    pub fn new(points: Vec<Vec2>) -> Self {
        Self { points, index: 0 }
    }

    /// Current target.
    pub fn current(&self) -> Option<Vec2> {
        self.points.get(self.index).copied()
    }

    /// Advance waypoint when close.
    pub fn advance_if_close(&mut self, pos: Vec2, threshold: f32) {
        if let Some(t) = self.current() {
            if (t - pos).length() < threshold {
                self.index = (self.index + 1) % self.points.len().max(1);
            }
        }
    }
}

/// Enemy AI state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyAi {
    /// Kind.
    pub kind: EnemyKind,
    /// FSM.
    pub fsm: StateMachine,
    /// Patrol.
    pub patrol: Option<PatrolPath>,
    /// Move speed.
    pub speed: f32,
    /// Chase speed.
    pub chase_speed: f32,
}

impl EnemyAi {
    /// Guard with patrol.
    pub fn guard(patrol: PatrolPath) -> Self {
        let mut fsm = StateMachine::new("patrol");
        fsm.allow("patrol", "alert");
        fsm.allow("alert", "chase");
        fsm.allow("chase", "patrol");
        fsm.allow("alert", "patrol");
        Self {
            kind: EnemyKind::Guard,
            fsm,
            patrol: Some(patrol),
            speed: 60.0,
            chase_speed: 110.0,
        }
    }

    /// Desired velocity given perception alert and last seen.
    pub fn desired_velocity(&mut self, pos: Vec2, alert: f32, last_seen: Option<Vec2>) -> Vec2 {
        if alert > 0.7 {
            let _ = self.fsm.try_transition("alert");
            let _ = self.fsm.try_transition("chase");
        } else if alert < 0.1 {
            let _ = self.fsm.try_transition("patrol");
        }

        match self.fsm.current.0.as_str() {
            "chase" | "alert" => {
                if let Some(t) = last_seen {
                    return (t - pos).normalize_or_zero() * self.chase_speed;
                }
                Vec2::ZERO
            }
            _ => {
                if let Some(patrol) = &mut self.patrol {
                    patrol.advance_if_close(pos, 8.0);
                    if let Some(t) = patrol.current() {
                        return (t - pos).normalize_or_zero() * self.speed;
                    }
                }
                Vec2::ZERO
            }
        }
    }
}

// Re-export BehaviorState usage
const _: Option<BehaviorState> = None;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patrol_then_chase() {
        let mut ai = EnemyAi::guard(PatrolPath::new(vec![Vec2::ZERO, Vec2::new(50.0, 0.0)]));
        let v = ai.desired_velocity(Vec2::ZERO, 0.0, None);
        assert!(v.x > 0.0);
        let v2 = ai.desired_velocity(Vec2::ZERO, 1.0, Some(Vec2::new(0.0, 100.0)));
        assert!(v2.y > 0.0);
        assert_eq!(ai.fsm.current.0, "chase");
    }
}
