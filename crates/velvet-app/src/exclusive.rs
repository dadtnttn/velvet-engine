//! Exclusive systems queue — systems that run alone with full `&mut App` access.

use std::collections::VecDeque;

use crate::system::{BoxedSystem, SystemId};

/// A queued exclusive system invocation.
pub struct ExclusiveCommand {
    /// System id (optional diagnostics).
    pub id: SystemId,
    /// Boxed system.
    pub system: BoxedSystem,
    /// Debug name.
    pub name: String,
}

impl std::fmt::Debug for ExclusiveCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExclusiveCommand")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

/// FIFO queue of exclusive systems to run at controlled points in the frame.
#[derive(Default)]
pub struct ExclusiveSystemQueue {
    queue: VecDeque<ExclusiveCommand>,
    next_id: u64,
    /// Max commands drained per flush (`None` = all).
    pub max_per_flush: Option<usize>,
    /// Total drained lifetime.
    drained_total: u64,
}

impl std::fmt::Debug for ExclusiveSystemQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExclusiveSystemQueue")
            .field("len", &self.queue.len())
            .field("max_per_flush", &self.max_per_flush)
            .field("drained_total", &self.drained_total)
            .finish()
    }
}

impl ExclusiveSystemQueue {
    /// Empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of pending commands.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Lifetime drained count.
    pub fn drained_total(&self) -> u64 {
        self.drained_total
    }

    /// Push a system function to run exclusively later.
    pub fn push<F>(&mut self, name: impl Into<String>, f: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut crate::app::App) + Send + Sync + 'static,
    {
        let id = SystemId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.queue.push_back(ExclusiveCommand {
            id,
            system: BoxedSystem::new(id, f),
            name: name.into(),
        });
        id
    }

    /// Push front (higher priority).
    pub fn push_front<F>(&mut self, name: impl Into<String>, f: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut crate::app::App) + Send + Sync + 'static,
    {
        let id = SystemId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.queue.push_front(ExclusiveCommand {
            id,
            system: BoxedSystem::new(id, f),
            name: name.into(),
        });
        id
    }

    /// Clear pending without running.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Pop one command without running.
    pub fn pop(&mut self) -> Option<ExclusiveCommand> {
        self.queue.pop_front()
    }

    /// Drain up to limit into a vec (for the app to run without holding borrow).
    pub fn drain(&mut self) -> Vec<ExclusiveCommand> {
        let limit = self.max_per_flush.unwrap_or(usize::MAX);
        let mut out = Vec::new();
        while out.len() < limit {
            match self.queue.pop_front() {
                Some(cmd) => {
                    self.drained_total = self.drained_total.saturating_add(1);
                    out.push(cmd);
                }
                None => break,
            }
        }
        out
    }
}

/// Run drained exclusive commands on the app.
pub fn run_exclusive_commands(app: &mut crate::app::App, commands: &mut [ExclusiveCommand]) {
    for cmd in commands.iter_mut() {
        cmd.system.run(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn queue_fifo() {
        let mut q = ExclusiveSystemQueue::new();
        q.push("a", |_| {});
        q.push("b", |_| {});
        assert_eq!(q.len(), 2);
        let a = q.pop().unwrap();
        assert_eq!(a.name, "a");
        assert_eq!(q.pop().unwrap().name, "b");
    }

    #[test]
    fn drain_and_run() {
        let mut app = App::new();
        app.insert_resource(0u32);
        let mut q = ExclusiveSystemQueue::new();
        q.push("inc", |app| {
            if let Some(v) = app.resource_mut::<u32>() {
                *v += 1;
            }
        });
        q.push("inc2", |app| {
            if let Some(v) = app.resource_mut::<u32>() {
                *v += 2;
            }
        });
        let mut cmds = q.drain();
        run_exclusive_commands(&mut app, &mut cmds);
        assert_eq!(*app.resource::<u32>().unwrap(), 3);
        assert_eq!(q.drained_total(), 2);
    }

    #[test]
    fn max_per_flush() {
        let mut q = ExclusiveSystemQueue::new();
        q.max_per_flush = Some(1);
        q.push("a", |_| {});
        q.push("b", |_| {});
        assert_eq!(q.drain().len(), 1);
        assert_eq!(q.len(), 1);
    }
}
