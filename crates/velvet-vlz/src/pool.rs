//! Reusable byte buffers for the stored (compressed) side of block reads, so steady streaming
//! does not allocate per block.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use std::sync::Mutex;

/// A bounded pool of `Vec<u8>`: buffers come back on drop while the pool holds less than
/// its byte budget; above it they are freed.
#[derive(Debug, Clone)]
pub struct BufferPool {
    inner: Arc<PoolInner>,
}

#[derive(Debug)]
struct PoolInner {
    free: Mutex<Vec<Vec<u8>>>,
    max_retained: usize,
    hits: AtomicU64,
    misses: AtomicU64,
}

/// Counters of a [`BufferPool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoolStats {
    /// Requests served by a pooled buffer.
    pub hits: u64,
    /// Requests that allocated.
    pub misses: u64,
    /// Bytes of capacity held right now.
    pub retained_bytes: usize,
}

impl BufferPool {
    /// A pool keeping at most `max_retained` bytes of idle capacity.
    pub fn new(max_retained: usize) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                free: Mutex::new(Vec::new()),
                max_retained,
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            }),
        }
    }

    /// A buffer of exactly `len` bytes (contents unspecified).
    pub fn take(&self, len: usize) -> PooledBuf {
        let reused = {
            let mut free = self.inner.free.lock().unwrap_or_else(|p| p.into_inner());
            // Smallest buffer that fits: keeps big buffers for big blocks.
            let best = free
                .iter()
                .enumerate()
                .filter(|(_, b)| b.capacity() >= len)
                .min_by_key(|(_, b)| b.capacity())
                .map(|(i, _)| i);
            best.map(|i| free.swap_remove(i))
        };
        let mut buf = match reused {
            Some(b) => {
                self.inner.hits.fetch_add(1, Ordering::Relaxed);
                b
            }
            None => {
                self.inner.misses.fetch_add(1, Ordering::Relaxed);
                Vec::with_capacity(len)
            }
        };
        buf.resize(len, 0);
        PooledBuf {
            buf,
            pool: Arc::clone(&self.inner),
        }
    }

    /// Current counters.
    pub fn stats(&self) -> PoolStats {
        let free = self.inner.free.lock().unwrap_or_else(|p| p.into_inner());
        PoolStats {
            hits: self.inner.hits.load(Ordering::Relaxed),
            misses: self.inner.misses.load(Ordering::Relaxed),
            retained_bytes: free.iter().map(Vec::capacity).sum(),
        }
    }
}

/// A buffer on loan from a [`BufferPool`].
#[derive(Debug)]
pub struct PooledBuf {
    buf: Vec<u8>,
    pool: Arc<PoolInner>,
}

impl Deref for PooledBuf {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> {
        &self.buf
    }
}

impl DerefMut for PooledBuf {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buf
    }
}

impl Drop for PooledBuf {
    fn drop(&mut self) {
        let buf = std::mem::take(&mut self.buf);
        if buf.capacity() == 0 {
            return;
        }
        let mut free = self.pool.free.lock().unwrap_or_else(|p| p.into_inner());
        let held: usize = free.iter().map(Vec::capacity).sum();
        if held + buf.capacity() <= self.pool.max_retained {
            free.push(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_within_budget() {
        let pool = BufferPool::new(1 << 20);
        drop(pool.take(1000));
        let b = pool.take(500);
        assert_eq!(b.len(), 500);
        drop(b);
        let s = pool.stats();
        assert_eq!((s.hits, s.misses), (1, 1));
        drop(pool.take(4 << 20)); // above the budget: freed, not kept
        assert!(pool.stats().retained_bytes < 1 << 20);
    }
}
