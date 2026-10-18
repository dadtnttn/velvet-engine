//! Ties the webview's processes to the host, so that they end when the host ends.
//!
//! WebView2 starts its own processes (the browser, the renderers and the helpers) as children of the
//! host. A host that closes normally shuts them down. A host that is killed rather than closed can
//! leave them running, and the browser then keeps the game's profile locked. So at start-up the host
//! puts itself in a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
//! The host holds the job's handle, and the system closes it when the host ends for any reason,
//! which ends every process left in the job. The webview's processes start after the host has joined
//! the job, so they join it too.

use std::io;
use std::mem::size_of;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// The job's handle, as a number so that a static can hold it. It stays open until the process ends.
/// Nothing closes it on purpose: the system does, and that ends the processes in the job.
static JOB: OnceLock<usize> = OnceLock::new();

/// Puts this process in a job object that ends its processes when the process ends. Call it before
/// the webview starts, so that the webview's processes join the job.
///
/// Calling it again does nothing. When it fails, the process runs on without the job, and the error
/// says why.
///
/// # Errors
///
/// Returns the Windows error when the job cannot be created, or when this process cannot join it.
pub fn end_with_host() -> io::Result<()> {
    if JOB.get().is_some() {
        return Ok(());
    }
    // SAFETY: an unnamed job with default security attributes; null is what both parameters take.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(io::Error::last_os_error());
    }
    match join(job) {
        Ok(()) => {
            // The host is one process, so this is the only job that is ever kept.
            let _ = JOB.set(job as usize);
            Ok(())
        }
        Err(error) => {
            // SAFETY: `job` came from CreateJobObjectW above and nothing else holds it.
            unsafe { CloseHandle(job) };
            Err(error)
        }
    }
}

/// Sets the kill-on-close limit on `job`, then assigns this process to it.
fn join(job: HANDLE) -> io::Result<()> {
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
        .map_err(io::Error::other)?;
    // SAFETY: `limits` is the structure that the information class names, and `size` is its size.
    let set = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size,
        )
    };
    if set == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: GetCurrentProcess gives a pseudo-handle to this process, which never needs closing.
    let assigned = unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) };
    if assigned == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_again_is_harmless() {
        end_with_host().expect("the test process joins a job");
        end_with_host().expect("a second call does nothing");
    }
}
