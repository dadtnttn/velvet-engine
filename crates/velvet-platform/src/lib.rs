//! # velvet-platform
//!
//! The platform layer of a Velvet Engine game that owns a winit event loop (a 3D game on
//! `velvet-render`, or `velvet-host2d`). The same game code runs on desktop and on Android:
//!
//! - **Entry point**: the game puts its whole program in a library function (`run()`); the
//!   desktop `main` calls it, and [`android_main!`] exports the Android entry that calls it too.
//!   [`event_loop`] builds winit's event loop for the platform (on Android, attached to the
//!   activity).
//! - **Files** ([`files`]): the game's read-only files through one API on every platform (a
//!   folder on desktop, the APK's assets on Android), and the writable folder for saves and
//!   settings.
//! - **GPU surface** ([`gpu`]): Android destroys the window's surface whenever the app leaves
//!   the screen. [`gpu::SurfaceSlot`] drops it on `suspended` and makes it again on `resumed`;
//!   [`gpu::instance_descriptor`] picks Vulkan with an OpenGL ES fallback on Android.
//! - **Screen** ([`screen`]): safe-area insets (notches, system bars), immersive full screen,
//!   orientation, keeping the screen on, the soft keyboard.
//! - **Touch** ([`touch`]): fingers to mouse events for games written for the mouse
//!   ([`touch::TouchMouse`]), the fingers themselves, and pinch, pan, tap and long-press gestures.
//! - **Lifecycle and keys** ([`lifecycle`], [`input`]): the back button, suspend and resume,
//!   focus and low-memory warnings as [`lifecycle::PlatformEvent`]s, and Android gamepad buttons.
//! - **Language** ([`system_locale`]): the user's language as a BCP 47 tag on every platform.
//!
//! ```ignore
//! // src/lib.rs of the game
//! pub fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let event_loop = velvet_platform::event_loop()?;
//!     event_loop.run_app(&mut MyGame::default())?;
//!     Ok(())
//! }
//! velvet_platform::android_main!(run);
//!
//! // src/main.rs of the game (desktop)
//! fn main() -> Result<(), Box<dyn std::error::Error>> { my_game::run() }
//! ```
//!
//! On Android the library is built as a `cdylib` (`velvet android build`, see
//! `docs/platforms/ANDROID.md`), packed into an APK with `android.app.NativeActivity` as its
//! activity, and started by the system through the `android_main` the macro exports.

#[cfg(target_os = "android")]
pub mod android;
mod error;
pub mod files;
pub mod gpu;
pub mod input;
pub mod lifecycle;
mod locale;
pub mod screen;
pub mod touch;

pub use error::PlatformError;
pub use lifecycle::{AppState, Lifecycle, PlatformEvent};
pub use locale::{posix_to_bcp47, system_locale};

use winit::event_loop::EventLoop;

/// The platform the game was built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Windows.
    Windows,
    /// Linux (and other desktop Unix systems).
    Linux,
    /// macOS.
    MacOs,
    /// Android phones and tablets.
    Android,
    /// iOS.
    Ios,
    /// The web (WebAssembly).
    Web,
}

impl Platform {
    /// The platform of this build.
    pub const fn current() -> Self {
        if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(target_os = "ios") {
            Self::Ios
        } else if cfg!(target_arch = "wasm32") {
            Self::Web
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }

    /// Phones and tablets: touch first, no mouse or keyboard to count on, a small and dense
    /// screen, a battery and a mobile GPU.
    pub const fn is_mobile(self) -> bool {
        matches!(self, Self::Android | Self::Ios)
    }
}

/// winit's event loop for this platform. On Android it is attached to the activity the system
/// started (the game must have been entered through [`android_main!`]).
pub fn event_loop() -> Result<EventLoop<()>, PlatformError> {
    event_loop_with_user_event::<()>()
}

/// [`event_loop`] with a user event type (for `EventLoopProxy` wake-ups).
pub fn event_loop_with_user_event<T: 'static>() -> Result<EventLoop<T>, PlatformError> {
    #[allow(unused_mut)]
    let mut builder = EventLoop::<T>::with_user_event();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let app = android::app().ok_or(PlatformError::NoAndroidApp)?;
        builder.with_android_app(app);
    }
    Ok(builder.build()?)
}

/// What a game's `run()` may return to [`android_main!`]: nothing, or a `Result` whose error is
/// written to the log.
pub trait MainResult {
    /// Writes a failure to the log (stderr reaches `logcat` on Android).
    fn report(self);
}

impl MainResult for () {
    fn report(self) {}
}

impl<T, E: std::fmt::Debug> MainResult for Result<T, E> {
    fn report(self) {
        if let Err(error) = self {
            eprintln!("[velvet-platform] the game ended with an error: {error:?}");
        }
    }
}

/// Exports the Android entry point (`android_main`) of a game whose program is the function
/// `$run` (the same one the desktop `main` calls). It expands to nothing on other platforms.
///
/// On Android the function runs on the activity's main loop thread. When it returns (the game
/// left its event loop), the activity is finished and, once Android has destroyed it, the
/// process ends: winit can build only one event loop per process, so the next launch must start
/// in a fresh one.
///
/// ```ignore
/// pub fn run() { /* build velvet_platform::event_loop() and run the game */ }
/// velvet_platform::android_main!(run);
/// ```
#[macro_export]
macro_rules! android_main {
    ($run:path) => {
        #[cfg(target_os = "android")]
        #[unsafe(no_mangle)]
        fn android_main(app: $crate::android::AndroidApp) {
            $crate::android::run_main(app, $run);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_of_this_build() {
        let p = Platform::current();
        assert!(!p.is_mobile() || cfg!(any(target_os = "android", target_os = "ios")));
        if cfg!(windows) {
            assert_eq!(p, Platform::Windows);
        }
    }

    #[test]
    fn main_results_report_without_panicking() {
        ().report();
        Ok::<(), String>(()).report();
        Err::<(), _>("broken").report();
    }
}
