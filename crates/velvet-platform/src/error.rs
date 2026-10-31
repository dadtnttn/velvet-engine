//! Errors of the platform layer.

use std::fmt;

/// Why the platform layer could not do what was asked.
#[derive(Debug)]
pub enum PlatformError {
    /// On Android: the game was not entered through `velvet_platform::android_main!`, so there
    /// is no activity to attach the event loop to.
    NoAndroidApp,
    /// winit could not build its event loop.
    EventLoop(winit::error::EventLoopError),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAndroidApp => write!(
                f,
                "no Android activity: start the game through velvet_platform::android_main!"
            ),
            Self::EventLoop(e) => write!(f, "the event loop could not be built: {e}"),
        }
    }
}

impl std::error::Error for PlatformError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoAndroidApp => None,
            Self::EventLoop(e) => Some(e),
        }
    }
}

impl From<winit::error::EventLoopError> for PlatformError {
    fn from(e: winit::error::EventLoopError) -> Self {
        Self::EventLoop(e)
    }
}
