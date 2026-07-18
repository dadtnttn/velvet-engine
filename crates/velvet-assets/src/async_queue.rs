//! Async load queue simulation (state machine) for tests and headless hosts.

use std::collections::VecDeque;

use crate::handle::AssetState;
use crate::path::AssetPath;

/// Priority for load requests (higher first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum LoadPriority {
    /// Background prefetch.
    Low = 0,
    /// Normal gameplay load.
    #[default]
    Normal = 1,
    /// Blocking / critical path.
    High = 2,
    /// Must complete before continuing (simulated).
    Critical = 3,
}

/// One item in the async load queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    /// Path to load.
    pub path: AssetPath,
    /// Priority.
    pub priority: LoadPriority,
    /// Simulated work units remaining.
    pub work_remaining: u32,
    /// Total work units (for progress).
    pub work_total: u32,
    /// State of this job.
    pub state: AssetState,
    /// Optional tag for batching/cancellation.
    pub tag: Option<String>,
    /// Monotonic job id.
    pub job_id: u64,
}

impl QueueItem {
    /// Progress 0..=1.
    pub fn progress(&self) -> f32 {
        if self.work_total == 0 {
            1.0
        } else {
            1.0 - (self.work_remaining as f32 / self.work_total as f32)
        }
    }
}

/// Queue phase for the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum QueuePhase {
    /// Accepting jobs, not processing.
    #[default]
    Idle,
    /// Processing active jobs.
    Running,
    /// Paused (keeps jobs).
    Paused,
    /// Draining: no new jobs, finish remaining.
    Draining,
}

/// Simulated async loader queue with concurrency limit.
#[derive(Debug, Clone)]
pub struct AsyncLoadQueue {
    pending: VecDeque<QueueItem>,
    active: Vec<QueueItem>,
    completed: Vec<QueueItem>,
    failed: Vec<QueueItem>,
    /// Max concurrent active jobs.
    pub max_concurrent: usize,
    /// Work units processed per tick across all active jobs.
    pub work_per_tick: u32,
    phase: QueuePhase,
    next_job_id: u64,
    /// Jobs accepted lifetime.
    accepted: u64,
}

impl Default for AsyncLoadQueue {
    fn default() -> Self {
        Self::new(4, 10)
    }
}

impl AsyncLoadQueue {
    /// Create with concurrency and work budget per tick.
    pub fn new(max_concurrent: usize, work_per_tick: u32) -> Self {
        Self {
            pending: VecDeque::new(),
            active: Vec::new(),
            completed: Vec::new(),
            failed: Vec::new(),
            max_concurrent: max_concurrent.max(1),
            work_per_tick: work_per_tick.max(1),
            phase: QueuePhase::Idle,
            next_job_id: 1,
            accepted: 0,
        }
    }

    /// Current phase.
    pub fn phase(&self) -> QueuePhase {
        self.phase
    }

    /// Start processing.
    pub fn start(&mut self) {
        if self.phase != QueuePhase::Draining {
            self.phase = QueuePhase::Running;
        }
    }

    /// Pause processing.
    pub fn pause(&mut self) {
        if self.phase == QueuePhase::Running {
            self.phase = QueuePhase::Paused;
        }
    }

    /// Resume from pause.
    pub fn resume(&mut self) {
        if self.phase == QueuePhase::Paused {
            self.phase = QueuePhase::Running;
        }
    }

    /// Stop accepting; finish active+pending.
    pub fn begin_drain(&mut self) {
        self.phase = QueuePhase::Draining;
    }

    /// Enqueue a load (returns job id). Fails if draining.
    pub fn enqueue(
        &mut self,
        path: AssetPath,
        priority: LoadPriority,
        work_units: u32,
        tag: Option<String>,
    ) -> Option<u64> {
        if self.phase == QueuePhase::Draining {
            return None;
        }
        let job_id = self.next_job_id;
        self.next_job_id = self.next_job_id.saturating_add(1);
        self.accepted = self.accepted.saturating_add(1);
        let work = work_units.max(1);
        let item = QueueItem {
            path,
            priority,
            work_remaining: work,
            work_total: work,
            state: AssetState::Loading,
            tag,
            job_id,
        };
        // Higher priority closer to front; among equals, append after existing equals (FIFO).
        let insert_at = self
            .pending
            .iter()
            .position(|p| p.priority < priority)
            .unwrap_or(self.pending.len());
        self.pending.insert(insert_at, item);
        if self.phase == QueuePhase::Idle {
            self.phase = QueuePhase::Running;
        }
        Some(job_id)
    }

    /// Convenience enqueue with normal priority and default work.
    pub fn enqueue_path(&mut self, path: impl Into<crate::path::VirtualPath>) -> Option<u64> {
        self.enqueue(AssetPath::virtual_path(path), LoadPriority::Normal, 5, None)
    }

    /// Pending count.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Active count.
    pub fn active_len(&self) -> usize {
        self.active.len()
    }

    /// Completed count (not yet drained).
    pub fn completed_len(&self) -> usize {
        self.completed.len()
    }

    /// Failed count.
    pub fn failed_len(&self) -> usize {
        self.failed.len()
    }

    /// Whether fully idle (nothing left).
    pub fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.active.is_empty()
    }

    /// Overall progress of incomplete work 0..=1 (1 if empty).
    pub fn overall_progress(&self) -> f32 {
        let items: Vec<&QueueItem> = self.pending.iter().chain(self.active.iter()).collect();
        if items.is_empty() {
            return 1.0;
        }
        let total: u32 = items.iter().map(|i| i.work_total).sum();
        let left: u32 = items.iter().map(|i| i.work_remaining).sum();
        if total == 0 {
            1.0
        } else {
            1.0 - left as f32 / total as f32
        }
    }

    /// Promote pending into active up to concurrency.
    fn fill_active(&mut self) {
        while self.active.len() < self.max_concurrent {
            let Some(item) = self.pending.pop_front() else {
                break;
            };
            self.active.push(item);
        }
    }

    /// Tick the simulation once; returns newly completed job ids this tick.
    pub fn tick(&mut self) -> Vec<u64> {
        if self.phase == QueuePhase::Paused || self.phase == QueuePhase::Idle {
            if self.phase == QueuePhase::Idle && !self.pending.is_empty() {
                self.phase = QueuePhase::Running;
            } else {
                return Vec::new();
            }
        }
        self.fill_active();
        if self.active.is_empty() {
            if self.phase == QueuePhase::Draining && self.pending.is_empty() {
                self.phase = QueuePhase::Idle;
            }
            return Vec::new();
        }

        let mut budget = self.work_per_tick;
        let mut i = 0;
        while i < self.active.len() && budget > 0 {
            // Distribute at most 1 work unit per active job while budget remains.
            let take = budget.min(self.active[i].work_remaining).min(1);
            if take == 0 {
                i += 1;
                continue;
            }
            self.active[i].work_remaining -= take;
            budget = budget.saturating_sub(take);
            i += 1;
        }
        // Second pass: dump remaining budget on first jobs.
        let mut j = 0;
        while budget > 0 && j < self.active.len() {
            let left = self.active[j].work_remaining;
            if left == 0 {
                j += 1;
                continue;
            }
            let take = budget.min(left);
            self.active[j].work_remaining -= take;
            budget -= take;
            j += 1;
        }

        let mut done_ids = Vec::new();
        let mut still = Vec::new();
        for mut item in self.active.drain(..) {
            if item.work_remaining == 0 {
                item.state = AssetState::Loaded;
                done_ids.push(item.job_id);
                self.completed.push(item);
            } else {
                still.push(item);
            }
        }
        self.active = still;
        self.fill_active();

        if self.phase == QueuePhase::Draining && self.is_idle() {
            self.phase = QueuePhase::Idle;
        }
        done_ids
    }

    /// Tick until idle or `max_ticks` (returns ticks executed).
    pub fn tick_until_idle(&mut self, max_ticks: u32) -> u32 {
        let mut n = 0;
        while !self.is_idle() && n < max_ticks {
            self.tick();
            n += 1;
        }
        n
    }

    /// Drain completed jobs.
    pub fn drain_completed(&mut self) -> Vec<QueueItem> {
        std::mem::take(&mut self.completed)
    }

    /// Drain failed.
    pub fn drain_failed(&mut self) -> Vec<QueueItem> {
        std::mem::take(&mut self.failed)
    }

    /// Fail a job by id (active or pending).
    pub fn fail_job(&mut self, job_id: u64) -> bool {
        if let Some(i) = self.pending.iter().position(|j| j.job_id == job_id) {
            let mut item = self.pending.remove(i).unwrap();
            item.state = AssetState::Failed;
            self.failed.push(item);
            return true;
        }
        if let Some(i) = self.active.iter().position(|j| j.job_id == job_id) {
            let mut item = self.active.remove(i);
            item.state = AssetState::Failed;
            self.failed.push(item);
            return true;
        }
        false
    }

    /// Cancel by tag (pending only).
    pub fn cancel_tag(&mut self, tag: &str) -> usize {
        let before = self.pending.len();
        self.pending.retain(|j| j.tag.as_deref() != Some(tag));
        before - self.pending.len()
    }

    /// Lifetime accepted jobs.
    pub fn accepted(&self) -> u64 {
        self.accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_with_priority() {
        let mut q = AsyncLoadQueue::new(1, 1);
        q.enqueue(AssetPath::virtual_path("a"), LoadPriority::Low, 10, None);
        q.enqueue(AssetPath::virtual_path("b"), LoadPriority::High, 10, None);
        q.start();
        // High should start first (and still be active after one unit of work).
        q.tick();
        assert_eq!(q.active[0].path.virtual_path.as_str(), "b");
        q.tick_until_idle(40);
        let done = q.drain_completed();
        assert_eq!(done.len(), 2);
        // High finishes before low with concurrency 1.
        assert_eq!(done[0].path.virtual_path.as_str(), "b");
        assert_eq!(done[1].path.virtual_path.as_str(), "a");
    }

    #[test]
    fn concurrency_limit() {
        let mut q = AsyncLoadQueue::new(2, 1);
        for i in 0..5 {
            q.enqueue(
                AssetPath::virtual_path(format!("f{i}")),
                LoadPriority::Normal,
                3,
                None,
            );
        }
        q.tick();
        assert_eq!(q.active_len(), 2);
        assert_eq!(q.pending_len(), 3);
    }

    #[test]
    fn pause_and_resume() {
        let mut q = AsyncLoadQueue::new(2, 10);
        q.enqueue_path("x");
        q.start();
        q.pause();
        assert!(q.tick().is_empty());
        q.resume();
        q.tick_until_idle(10);
        assert_eq!(q.drain_completed().len(), 1);
    }

    #[test]
    fn drain_stops_enqueue() {
        let mut q = AsyncLoadQueue::new(2, 10);
        q.enqueue_path("a");
        q.begin_drain();
        assert!(q.enqueue_path("b").is_none());
        q.tick_until_idle(10);
        assert!(q.is_idle());
    }

    #[test]
    fn fail_and_cancel() {
        let mut q = AsyncLoadQueue::new(2, 1);
        q.enqueue(
            AssetPath::virtual_path("f"),
            LoadPriority::Normal,
            10,
            Some("batch".into()),
        );
        q.enqueue(
            AssetPath::virtual_path("g"),
            LoadPriority::Normal,
            10,
            Some("batch".into()),
        );
        assert_eq!(q.cancel_tag("batch"), 2);
        let mut q = AsyncLoadQueue::new(2, 1);
        let id = q
            .enqueue(AssetPath::virtual_path("f"), LoadPriority::Normal, 10, None)
            .unwrap();
        q.tick();
        assert!(q.fail_job(id));
        assert_eq!(q.drain_failed().len(), 1);
    }
}
