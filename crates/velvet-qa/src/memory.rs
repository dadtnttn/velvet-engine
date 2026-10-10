//! The process's memory, for leak hunts.

/// The process's private memory in MB (what it has committed); 0 where it cannot be asked
/// (off Windows).
pub fn private_mb() -> u64 {
    #[cfg(windows)]
    // SAFETY: the counters are a plain struct the call fills in, sized by `cb`.
    unsafe {
        use windows_sys::Win32::System::ProcessStatus::{
            K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters as *mut PROCESS_MEMORY_COUNTERS_EX as *mut _,
            counters.cb,
        ) != 0
        {
            return (counters.PrivateUsage / (1024 * 1024)) as u64;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn a_running_process_has_private_memory() {
        let held = vec![1u8; 64 << 20];
        assert!(super::private_mb() >= 64, "{}", super::private_mb());
        drop(std::hint::black_box(held));
    }
}
