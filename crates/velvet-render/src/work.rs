//! A few threads that stay, for the work a texture is split into.
//!
//! Every thread made and ended leaves a little memory behind on Windows (the system and the
//! graphics driver keep data for each thread they have seen: a few hundred bytes a thread). The
//! mip chains and the BC7 blocks of every texture used to be made on fresh threads, up to sixteen
//! for each level: thousands for a world, a megabyte or more lost each time one was built again.
//! These threads are made once, the first time there is work for them, and kept.

use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Pool {
    sender: Mutex<mpsc::Sender<Job>>,
    threads: usize,
}

thread_local! {
    /// Set on the pool's own threads (work split from one of them runs in turn there).
    static IN_POOL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(2, 16);
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        for k in 0..threads {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name(format!("velvet-work-{k}"))
                .spawn(move || {
                    IN_POOL.with(|p| p.set(true));
                    loop {
                        let job = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => break,
                        }
                    }
                })
                .expect("a worker thread");
        }
        Pool {
            sender: Mutex::new(sender),
            threads,
        }
    })
}

/// How many threads the pool has: how many parts work is best split into.
pub(crate) fn threads() -> usize {
    pool().threads
}

/// Runs `work` on every part, on the pool's threads, and returns once all are done (a panic in
/// one is raised here). On one of the pool's own threads the parts run in turn (it would wait on
/// itself).
pub(crate) fn each<'a, T: Send + 'a>(parts: Vec<T>, work: &'a (dyn Fn(T) + Sync + 'a)) {
    if parts.len() <= 1 || IN_POOL.with(|p| p.get()) {
        for part in parts {
            work(part);
        }
        return;
    }
    // (parts not finished yet, whether one panicked)
    let pending = Arc::new((Mutex::new((parts.len(), false)), Condvar::new()));
    let sender = pool()
        .sender
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    for part in parts {
        let pending = pending.clone();
        let job: Box<dyn FnOnce() + Send + 'a> = Box::new(move || {
            let failed =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(part))).is_err();
            let (state, done) = &*pending;
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            state.0 -= 1;
            state.1 |= failed;
            if state.0 == 0 {
                done.notify_all();
            }
        });
        // SAFETY: the job borrows `work` and its part for `'a`, and this call does not return
        // before every job it handed over has run to its end (a panic in it is caught above and
        // counted), so nothing it borrows is gone while it runs. Only the lifetime is changed.
        let job: Job = unsafe { std::mem::transmute::<Box<dyn FnOnce() + Send + 'a>, Job>(job) };
        if let Err(mpsc::SendError(job)) = sender.send(job) {
            job();
        }
    }
    let (state, done) = &*pending;
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    while state.0 > 0 {
        state = done.wait(state).unwrap_or_else(|e| e.into_inner());
    }
    assert!(!state.1, "a part of the work panicked");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_is_done_once_on_the_kept_threads() {
        let mut data = vec![0u32; 1000];
        let parts: Vec<(usize, &mut [u32])> = data.chunks_mut(37).enumerate().collect();
        each(parts, &|(k, chunk): (usize, &mut [u32])| {
            for (i, v) in chunk.iter_mut().enumerate() {
                *v += (k * 37 + i) as u32 + 1;
            }
        });
        assert!(data.iter().enumerate().all(|(i, v)| *v == i as u32 + 1));
        // Again: the same threads take it.
        let before = threads();
        each((0..64).collect(), &|_: i32| {});
        assert_eq!(threads(), before);
    }

    #[test]
    fn work_split_again_inside_a_part_still_finishes() {
        let total = std::sync::atomic::AtomicUsize::new(0);
        each((0..8).collect(), &|_: i32| {
            each((0..8).collect(), &|_: i32| {
                total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            });
        });
        assert_eq!(total.into_inner(), 64);
    }

    #[test]
    #[should_panic(expected = "a part of the work panicked")]
    fn a_panic_in_a_part_reaches_the_caller() {
        each((0..8).collect(), &|k: i32| assert!(k != 5));
    }
}
