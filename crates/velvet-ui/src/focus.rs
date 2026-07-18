//! Keyboard / gamepad focus navigation.

use crate::node::NodeId;

/// Focus navigation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDir {
    /// Next.
    Next,
    /// Previous.
    Prev,
    /// Up.
    Up,
    /// Down.
    Down,
    /// Left.
    Left,
    /// Right.
    Right,
}

/// Focus state.
#[derive(Debug, Clone, Default)]
pub struct FocusState {
    /// Focused node.
    pub focused: Option<NodeId>,
    /// Tab order.
    pub order: Vec<NodeId>,
}

impl FocusState {
    /// Set tab order.
    pub fn set_order(&mut self, order: Vec<NodeId>) {
        self.order = order;
        if let Some(f) = self.focused {
            if !self.order.contains(&f) {
                self.focused = self.order.first().copied();
            }
        } else {
            self.focused = self.order.first().copied();
        }
    }

    /// Move focus.
    pub fn move_focus(&mut self, dir: FocusDir) {
        if self.order.is_empty() {
            self.focused = None;
            return;
        }
        let idx = self
            .focused
            .and_then(|f| self.order.iter().position(|id| *id == f))
            .unwrap_or(0);
        let next = match dir {
            FocusDir::Next | FocusDir::Down | FocusDir::Right => (idx + 1) % self.order.len(),
            FocusDir::Prev | FocusDir::Up | FocusDir::Left => {
                if idx == 0 {
                    self.order.len() - 1
                } else {
                    idx - 1
                }
            }
        };
        self.focused = Some(self.order[next]);
    }

    /// Activate focused (for tests).
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_next() {
        let mut f = FocusState::default();
        f.set_order(vec![NodeId(1), NodeId(2), NodeId(3)]);
        f.move_focus(FocusDir::Next);
        assert_eq!(f.focused, Some(NodeId(2)));
        f.move_focus(FocusDir::Next);
        f.move_focus(FocusDir::Next);
        assert_eq!(f.focused, Some(NodeId(1)));
    }
}
