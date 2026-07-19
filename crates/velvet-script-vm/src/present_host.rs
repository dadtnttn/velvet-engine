//! Presentation host state for VS3 natives (`show`, `set_bg`, `ui_flag`, …).
//!
//! **State only** — no drawing. The game host mirrors this into
//! `velvet-story::PresentationState` / GPU presenters.

use std::cell::RefCell;
use std::collections::BTreeMap;

use indexmap::IndexMap;

/// One sprite requested by script logics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentSprite {
    /// Character / sprite id.
    pub id: String,
    /// Expression tag (optional).
    pub expression: Option<String>,
    /// Placement tag (left/center/right/…).
    pub at: Option<String>,
}

/// Host presentation state mutated by presentation natives.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresentHostState {
    /// Current background path (virtual asset path).
    pub background: Option<String>,
    /// Visible sprites keyed by id (insertion order).
    pub sprites: IndexMap<String, PresentSprite>,
    /// UI flags (say box, choice menu, language menu, …).
    pub ui_flags: BTreeMap<String, bool>,
    /// Ordered log of host ops (debug / tests).
    pub log: Vec<String>,
}

impl PresentHostState {
    /// Empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// `show(id [, expression [, at]])`
    pub fn show(
        &mut self,
        id: impl Into<String>,
        expression: Option<String>,
        at: Option<String>,
    ) {
        let id = id.into();
        self.log.push(format!(
            "show {id} expr={} at={}",
            expression.as_deref().unwrap_or("-"),
            at.as_deref().unwrap_or("-")
        ));
        self.sprites.insert(
            id.clone(),
            PresentSprite {
                id,
                expression,
                at,
            },
        );
    }

    /// `hide(id)`
    pub fn hide(&mut self, id: &str) {
        self.sprites.shift_remove(id);
        self.log.push(format!("hide {id}"));
    }

    /// `set_bg(path)`
    pub fn set_bg(&mut self, path: impl Into<String>) {
        let path = path.into();
        self.log.push(format!("set_bg {path}"));
        self.background = Some(path);
    }

    /// `ui_flag(name, on)`
    pub fn set_ui_flag(&mut self, name: impl Into<String>, on: bool) {
        let name = name.into();
        self.log.push(format!("ui_flag {name}={on}"));
        self.ui_flags.insert(name, on);
    }

    /// `ui_flag_get(name)`
    pub fn ui_flag(&self, name: &str) -> bool {
        self.ui_flags.get(name).copied().unwrap_or(false)
    }

    /// Clear all presentation state.
    pub fn clear(&mut self) {
        self.background = None;
        self.sprites.clear();
        self.ui_flags.clear();
        self.log.clear();
    }
}

thread_local! {
    static PRESENT: RefCell<PresentHostState> = RefCell::new(PresentHostState::new());
}

/// Run `f` with a fresh presentation host, then return the resulting state.
pub fn with_present_host<R>(f: impl FnOnce() -> R) -> (R, PresentHostState) {
    PRESENT.with(|cell| {
        *cell.borrow_mut() = PresentHostState::new();
    });
    let r = f();
    let state = PRESENT.with(|cell| cell.borrow().clone());
    (r, state)
}

/// Replace the thread-local host (e.g. continue a session).
pub fn install_present_host(state: PresentHostState) {
    PRESENT.with(|cell| {
        *cell.borrow_mut() = state;
    });
}

/// Snapshot current host state.
pub fn take_present_host() -> PresentHostState {
    PRESENT.with(|cell| cell.borrow().clone())
}

/// Mutate the thread-local host.
pub fn present_host_mut<R>(f: impl FnOnce(&mut PresentHostState) -> R) -> R {
    PRESENT.with(|cell| f(&mut cell.borrow_mut()))
}

/// Reset host to empty.
pub fn reset_present_host() {
    PRESENT.with(|cell| {
        *cell.borrow_mut() = PresentHostState::new();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_set_bg_ui_flags() {
        let mut h = PresentHostState::new();
        h.set_bg("bg/station.png");
        h.show("nora", Some("happy".into()), Some("left".into()));
        h.set_ui_flag("say_visible", true);
        h.set_ui_flag("choice_open", false);
        assert_eq!(h.background.as_deref(), Some("bg/station.png"));
        assert_eq!(h.sprites["nora"].expression.as_deref(), Some("happy"));
        assert!(h.ui_flag("say_visible"));
        assert!(!h.ui_flag("choice_open"));
        assert!(!h.ui_flag("missing"));
        h.hide("nora");
        assert!(h.sprites.is_empty());
        assert!(h.log.iter().any(|l| l.starts_with("show")));
        assert!(h.log.iter().any(|l| l.starts_with("set_bg")));
    }

    #[test]
    fn with_present_host_isolates() {
        let ((), state) = with_present_host(|| {
            present_host_mut(|h| {
                h.set_bg("a.png");
                h.show("hero", None, None);
            });
        });
        assert_eq!(state.background.as_deref(), Some("a.png"));
        assert!(state.sprites.contains_key("hero"));
    }
}
