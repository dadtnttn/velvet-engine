#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

//! Optional world-editor preview. The shipped game embeds this scene itself.
#[path = "../exploration3d.rs"]
#[allow(dead_code)] // The preview does not use the embedded-host/test entry points.
mod exploration3d;
#[path = "../exploration3d_state.rs"]
mod exploration3d_state;
#[path = "../window_display.rs"]
#[allow(dead_code)] // The preview presents only 3D, not the VN canvas.
mod window_display;

fn main() -> anyhow::Result<()> {
    velvet_avif::register_image_hooks();
    exploration3d::prefs::load();
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::*;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    exploration3d::run_standalone()
}
