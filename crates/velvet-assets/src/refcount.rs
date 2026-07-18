//! Reference-counted strong handles and live tracking.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::handle::Handle;

/// Shared refcount table keyed by asset id.
#[derive(Debug, Default, Clone)]
pub struct RefCountTable {
    inner: Arc<Mutex<HashMap<u64, u32>>>,
}

impl RefCountTable {
    /// Create.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current count for raw id.
    pub fn count(&self, raw: u64) -> u32 {
        self.inner.lock().get(&raw).copied().unwrap_or(0)
    }

    /// Increment; returns new count.
    pub fn retain(&self, raw: u64) -> u32 {
        let mut g = self.inner.lock();
        let e = g.entry(raw).or_insert(0);
        *e = e.saturating_add(1);
        *e
    }

    /// Decrement; returns new count (0 if removed).
    pub fn release(&self, raw: u64) -> u32 {
        let mut g = self.inner.lock();
        let Some(e) = g.get_mut(&raw) else {
            return 0;
        };
        *e = e.saturating_sub(1);
        let v = *e;
        if v == 0 {
            g.remove(&raw);
        }
        v
    }

    /// Number of tracked assets with count > 0.
    pub fn live_assets(&self) -> usize {
        self.inner.lock().len()
    }

    /// Clear all counts.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

/// Strong handle that increments a refcount table on clone and decrements on drop.
#[derive(Debug)]
pub struct RcHandle<T> {
    handle: Handle<T>,
    raw: u64,
    table: RefCountTable,
    _marker: PhantomData<fn() -> T>,
}

impl<T> RcHandle<T> {
    /// Create and retain.
    pub fn new(handle: Handle<T>, raw_id: u64, table: RefCountTable) -> Self {
        table.retain(raw_id);
        Self {
            handle,
            raw: raw_id,
            table,
            _marker: PhantomData,
        }
    }

    /// From handle generation as raw key (pair with explicit raw when possible).
    pub fn from_handle(handle: Handle<T>, table: RefCountTable) -> Self {
        let raw = handle.generation.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self::new(handle, raw, table)
    }

    /// Weak handle view.
    pub fn handle(&self) -> Handle<T> {
        self.handle
    }

    /// Raw refcount key.
    pub fn raw_id(&self) -> u64 {
        self.raw
    }

    /// Current refcount.
    pub fn strong_count(&self) -> u32 {
        self.table.count(self.raw)
    }

    /// Downgrade to weak handle only (drops strong on drop of self still).
    pub fn downgrade(&self) -> Handle<T> {
        self.handle
    }
}

impl<T> Clone for RcHandle<T> {
    fn clone(&self) -> Self {
        self.table.retain(self.raw);
        Self {
            handle: self.handle,
            raw: self.raw,
            table: self.table.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for RcHandle<T> {
    fn drop(&mut self) {
        self.table.release(self.raw);
    }
}

/// Tracks which assets are live (strong_count > 0) for GC policies.
#[derive(Debug, Default, Clone)]
pub struct LiveSet {
    table: RefCountTable,
    /// raw → last used frame
    last_used: Arc<Mutex<HashMap<u64, u64>>>,
}

impl LiveSet {
    /// Create.
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow table for RcHandle construction.
    pub fn table(&self) -> RefCountTable {
        self.table.clone()
    }

    /// Touch asset as used this frame.
    pub fn touch(&self, raw: u64, frame: u64) {
        self.last_used.lock().insert(raw, frame);
        if self.table.count(raw) == 0 {
            self.table.retain(raw);
        }
    }

    /// Collect raw ids with zero refcount or unused since `min_frame`.
    pub fn collect_unused(&self, min_frame: u64) -> Vec<u64> {
        let used = self.last_used.lock();
        let counts = self.table.inner.lock();
        let mut out = Vec::new();
        for (&raw, &count) in counts.iter() {
            let last = used.get(&raw).copied().unwrap_or(0);
            if count == 0 || last < min_frame {
                out.push(raw);
            }
        }
        // Also last_used entries with no count
        for (&raw, &last) in used.iter() {
            if last < min_frame && !counts.contains_key(&raw) {
                out.push(raw);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Live count.
    pub fn live_count(&self) -> usize {
        self.table.live_assets()
    }
}

/// Expanded StrongHandle with optional table integration.
#[derive(Debug)]
pub struct TrackedStrongHandle<T> {
    /// Inner handle.
    pub handle: Handle<T>,
    rc: Option<RcHandle<T>>,
}

impl<T> TrackedStrongHandle<T> {
    /// Untracked (legacy) strong handle.
    pub fn untracked(handle: Handle<T>) -> Self {
        Self { handle, rc: None }
    }

    /// Tracked strong handle.
    pub fn tracked(handle: Handle<T>, table: RefCountTable) -> Self {
        let rc = RcHandle::from_handle(handle, table);
        Self {
            handle,
            rc: Some(rc),
        }
    }

    /// Strong count if tracked.
    pub fn strong_count(&self) -> Option<u32> {
        self.rc.as_ref().map(|r| r.strong_count())
    }

    /// Weak handle.
    pub fn weak(&self) -> Handle<T> {
        self.handle
    }
}

impl<T> Clone for TrackedStrongHandle<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle,
            rc: self.rc.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::{next_generation, AssetId};

    #[test]
    fn refcount_clone_drop() {
        let table = RefCountTable::new();
        let h = Handle::<()>::new(AssetId::default(), next_generation());
        {
            let a = RcHandle::new(h, 42, table.clone());
            assert_eq!(a.strong_count(), 1);
            {
                let b = a.clone();
                assert_eq!(b.strong_count(), 2);
            }
            assert_eq!(a.strong_count(), 1);
        }
        assert_eq!(table.count(42), 0);
        assert_eq!(table.live_assets(), 0);
    }

    #[test]
    fn tracked_handle() {
        let table = RefCountTable::new();
        let h = Handle::<u8>::new(AssetId::default(), next_generation());
        let t = TrackedStrongHandle::tracked(h, table.clone());
        assert!(t.strong_count().unwrap() >= 1);
        let t2 = t.clone();
        assert!(t2.strong_count().unwrap() >= 2);
    }

    #[test]
    fn live_set_touch() {
        let live = LiveSet::new();
        live.touch(7, 10);
        assert_eq!(live.live_count(), 1);
        let unused = live.collect_unused(11);
        assert!(unused.contains(&7));
    }
}
