//! Layout transitions: FLIP (reorder) + enter/exit (add/remove).
//!
//! FLIP = First, Last, Invert, Play: record positions, mutate + layout,
//! then animate the delta instead of teleporting. Enter/exit tracks per-id
//! fade clocks so added rows fade+slide in and removed rows fade out
//! before hosts drop them. All state is plain data hosts tick.

use std::collections::HashMap;

use velvet_math::Vec2;

use crate::tree::UiTree;

/// Snapshot of node positions by id.
pub type PositionSnapshot = HashMap<String, Vec2>;

/// Record current node positions by name (first same-name node wins).
pub fn capture_positions(tree: &UiTree) -> PositionSnapshot {
    let mut out = PositionSnapshot::new();
    tree.for_each_node(&mut |node| {
        out.entry(node.name.clone()).or_insert(node.rect.pos);
    });
    out
}

/// One FLIP delta: animate `from` → current position.
#[derive(Debug, Clone, PartialEq)]
pub struct FlipDelta {
    /// Node name.
    pub id: String,
    /// Position at capture time.
    pub from: Vec2,
    /// Position now.
    pub to: Vec2,
}

/// Deltas for names present in both snapshots whose position changed.
pub fn flip_deltas(before: &PositionSnapshot, tree: &UiTree) -> Vec<FlipDelta> {
    let mut out = Vec::new();
    tree.for_each_node(&mut |node| {
        if let Some(from) = before.get(&node.name) {
            if *from != node.rect.pos {
                out.push(FlipDelta {
                    id: node.name.clone(),
                    from: *from,
                    to: node.rect.pos,
                });
            }
        }
    });
    out
}

/// Enter/exit fade clocks by node name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppearTracker {
    /// Name → 0..=1 progress (entering up, exiting down).
    entering: HashMap<String, f32>,
    /// Name → 0..=1 remaining.
    exiting: HashMap<String, f32>,
    /// Seconds for a full fade.
    pub duration: f32,
}

impl AppearTracker {
    /// New tracker with fade duration.
    pub fn with_duration(duration: f32) -> Self {
        Self {
            duration: duration.max(1e-4),
            ..Default::default()
        }
    }

    /// Mark names as entering (fade 0 → 1).
    pub fn enter(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            self.exiting.remove(&name);
            self.entering.insert(name, 0.0);
        }
    }

    /// Mark names as exiting (fade 1 → 0, pruned at 0).
    pub fn exit(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            self.entering.remove(&name);
            self.exiting.insert(name, 1.0);
        }
    }

    /// Advance clocks. Returns names whose exit fade finished this tick
    /// (hosts drop them now).
    pub fn tick(&mut self, dt: f32) -> Vec<String> {
        let k = dt.max(0.0) / self.duration;
        for p in self.entering.values_mut() {
            *p = (*p + k).min(1.0);
        }
        for p in self.exiting.values_mut() {
            *p = (*p - k).max(0.0);
        }
        self.entering.retain(|_, p| *p < 1.0);
        let mut done = Vec::new();
        self.exiting.retain(|name, p| {
            let live = *p > 0.0;
            if !live {
                done.push(name.clone());
            }
            live
        });
        done
    }

    /// Current alpha for a name (1 when untracked).
    pub fn alpha(&self, name: &str) -> f32 {
        if let Some(p) = self.entering.get(name) {
            return p.clamp(0.0, 1.0);
        }
        if let Some(p) = self.exiting.get(name) {
            return p.clamp(0.0, 1.0);
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;
    use velvet_math::Vec2;

    fn tree_with_rows(names: &[&str]) -> UiTree {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        {
            let node = ui.get_mut(root).unwrap();
            node.layout = crate::layout::LayoutType::Column { wrap: false };
        }
        for name in names {
            ui.add_node(
                *name,
                WidgetKind::Label {
                    text: (*name).into(),
                },
                Some(root),
            );
        }
        ui.layout(Vec2::new(200.0, 400.0));
        ui
    }

    #[test]
    fn flip_detects_reorder() {
        let mut ui = tree_with_rows(&["a", "b", "c"]);
        let before = capture_positions(&ui);
        // Reverse visual order by rebuilding children (same names).
        let root = ui.root().unwrap();
        let children = ui.get(root).unwrap().children.clone();
        ui.get_mut(root).unwrap().children = children.into_iter().rev().collect();
        ui.layout(Vec2::new(200.0, 400.0));
        let deltas = flip_deltas(&before, &ui);
        assert_eq!(deltas.len(), 2, "{deltas:?}");
        let a = deltas.iter().find(|d| d.id == "a").unwrap();
        assert!(a.to.y > a.from.y);
        let c = deltas.iter().find(|d| d.id == "c").unwrap();
        assert!(c.to.y < c.from.y);
        // The middle row did not move: no delta for it.
        assert!(deltas.iter().all(|d| d.id != "b"));
    }

    #[test]
    fn appear_tracks_enter_exit() {
        let mut tracker = AppearTracker::with_duration(0.5);
        assert_eq!(tracker.alpha("x"), 1.0);
        tracker.enter(["x".to_string()]);
        assert_eq!(tracker.alpha("x"), 0.0);
        tracker.tick(0.25);
        assert!((tracker.alpha("x") - 0.5).abs() < 1e-4);
        tracker.tick(1.0);
        assert_eq!(tracker.alpha("x"), 1.0);
        tracker.exit(["x".to_string()]);
        assert_eq!(tracker.alpha("x"), 1.0);
        tracker.tick(0.25);
        assert!((tracker.alpha("x") - 0.5).abs() < 1e-4);
        // Finishing prunes the entry (alpha reads untracked = 1.0) and
        // reports the name once for hosts to drop the row.
        let done = tracker.tick(0.5);
        assert_eq!(done, vec!["x".to_string()]);
        assert_eq!(tracker.alpha("x"), 1.0);
        assert!(tracker.tick(0.5).is_empty());
    }
}
