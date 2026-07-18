//! Interaction prompts (nearest interactable in range).

use velvet_math::Vec2;

use crate::components::Interactable;

/// Interaction event when player confirms near a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractEvent {
    /// Target entity id (host-defined usize / ECS key).
    pub target_id: usize,
    /// Action string.
    pub action: String,
}

/// Finds nearest enabled interactable within range.
pub struct InteractionSystem;

impl InteractionSystem {
    /// Query nearest.
    pub fn nearest(
        player_pos: Vec2,
        candidates: &[(usize, Vec2, &Interactable)],
    ) -> Option<(usize, f32, String)> {
        let mut best: Option<(usize, f32, String)> = None;
        for (id, pos, inter) in candidates {
            if !inter.enabled {
                continue;
            }
            let d = (*pos - player_pos).length();
            if d <= inter.radius && best.as_ref().map(|(_, bd, _)| d < *bd).unwrap_or(true) {
                best = Some((*id, d, inter.action.clone()));
            }
        }
        best
    }

    /// If confirm pressed and nearest exists, build event.
    pub fn try_interact(
        player_pos: Vec2,
        candidates: &[(usize, Vec2, &Interactable)],
        confirm: bool,
    ) -> Option<InteractEvent> {
        if !confirm {
            return None;
        }
        let (target_id, _, action) = Self::nearest(player_pos, candidates)?;
        Some(InteractEvent { target_id, action })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Interactable;

    #[test]
    fn picks_nearest() {
        let a = Interactable::new("door", 32.0);
        let b = Interactable::new("chest", 32.0);
        let list = [
            (1usize, Vec2::new(40.0, 0.0), &a),
            (2usize, Vec2::new(10.0, 0.0), &b),
        ];
        let (id, _, action) = InteractionSystem::nearest(Vec2::ZERO, &list).unwrap();
        assert_eq!(id, 2);
        assert_eq!(action, "chest");
    }
}
