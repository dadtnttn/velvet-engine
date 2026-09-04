//! UI sound events: the engine emits, the host plays.
//!
//! [`UiTree::process`] pushes [`UiSound`] requests (hover ticks, clicks,
//! blocked clicks on disabled widgets, picks, tab selects). Hosts drain
//! the queue each frame into `velvet-audio` (or any bus) and map event
//! kinds to clips. Nothing plays by itself — headless runs just log.

/// One UI sound request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSound {
    /// Event kind (`hover`, `click`, `blocked`, `pick`, `select`, `toggle`).
    pub kind: UiSoundKind,
    /// Node name that caused it (empty when none).
    pub node: String,
}

/// UI sound event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiSoundKind {
    /// Pointer entered an enabled interactive widget.
    #[default]
    Hover,
    /// Activated (click release / confirm on Button).
    Click,
    /// Clicked while disabled.
    Blocked,
    /// Dropdown option picked.
    Pick,
    /// Tab selected.
    Select,
    /// Toggle flipped.
    Toggle,
}

impl UiSoundKind {
    /// Stable name for hosts mapping kinds to clips.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Click => "click",
            Self::Blocked => "blocked",
            Self::Pick => "pick",
            Self::Select => "select",
            Self::Toggle => "toggle",
        }
    }
}

/// Drainable UI sound queue (one per tree, cleared by hosts per frame).
#[derive(Debug, Clone, Default)]
pub struct UiSoundQueue {
    events: Vec<UiSound>,
}

impl UiSoundQueue {
    /// Empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push an event.
    pub fn push(&mut self, kind: UiSoundKind, node: impl Into<String>) {
        self.events.push(UiSound {
            kind,
            node: node.into(),
        });
    }

    /// Drain all pending events (hosts call once per frame).
    pub fn drain(&mut self) -> Vec<UiSound> {
        std::mem::take(&mut self.events)
    }

    /// Pending count.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_empties_in_order() {
        let mut queue = UiSoundQueue::new();
        assert!(queue.is_empty());
        queue.push(UiSoundKind::Hover, "a");
        queue.push(UiSoundKind::Click, "b");
        assert_eq!(queue.len(), 2);
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].kind, UiSoundKind::Hover);
        assert_eq!(drained[0].node, "a");
        assert_eq!(UiSoundKind::Blocked.as_str(), "blocked");
        assert!(queue.is_empty());
    }
}
