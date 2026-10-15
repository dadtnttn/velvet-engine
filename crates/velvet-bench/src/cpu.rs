//! Process CPU time (user + kernel, all threads), for CPU-cost figures.

/// Seconds of CPU the whole process has used, or `None` where not supported.
#[cfg(windows)]
pub fn process_cpu_seconds() -> Option<f64> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let (mut c, mut e, mut k, mut u) = Default::default();
    // SAFETY: the pseudo-handle of the current process is always valid; the out-pointers are
    // to live, properly laid out FILETIME structs.
    let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) };
    if ok == 0 {
        return None;
    }
    let ticks = |t: &FileTime| (u64::from(t.high) << 32 | u64::from(t.low)) as f64;
    Some((ticks(&k) + ticks(&u)) / 1e7)
}

/// Seconds of CPU the whole process has used, or `None` where not supported.
#[cfg(unix)]
pub fn process_cpu_seconds() -> Option<f64> {
    // SAFETY: getrusage fills the struct it is given.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return None;
    }
    let secs = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1e6;
    Some(secs(usage.ru_utime) + secs(usage.ru_stime))
}

/// Seconds of CPU the whole process has used, or `None` where not supported.
#[cfg(not(any(windows, unix)))]
pub fn process_cpu_seconds() -> Option<f64> {
    None
}
