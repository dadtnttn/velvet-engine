//! The screen: safe-area insets, immersive full screen, orientation, keeping it on, and the soft
//! keyboard. On desktop these are no-ops (and the insets are zero); full screen there is winit's
//! `Window::set_fullscreen`.
//!
//! Sizes are in window (physical) pixels, the same space as winit's `inner_size()` and touch
//! positions. Scale UI with `Window::scale_factor()` (on Android: the screen density / 160).

/// Space at each edge of the window that system UI covers or may cover: status and navigation
/// bars that are showing, and display cutouts (notches, camera holes). Keep buttons and text
/// inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Insets {
    /// Pixels at the left edge.
    pub left: u32,
    /// Pixels at the top edge.
    pub top: u32,
    /// Pixels at the right edge.
    pub right: u32,
    /// Pixels at the bottom edge.
    pub bottom: u32,
}

impl Insets {
    /// No insets.
    pub const ZERO: Insets = Insets {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    /// Whether every side is zero.
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    /// The larger inset of the two on each side.
    pub fn max(self, other: Insets) -> Insets {
        Insets {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// The safe rectangle `(x, y, width, height)` of a `width`×`height` window.
    pub fn safe_rect(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let x = self.left.min(width);
        let y = self.top.min(height);
        let w = width.saturating_sub(self.left + self.right);
        let h = height.saturating_sub(self.top + self.bottom);
        (x, y, w, h)
    }
}

/// How the game wants the screen turned. The default comes from the Android manifest
/// (`velvet android` writes it from the game's `orientation` setting); [`set_orientation`]
/// changes it at run time. Android 16 ignores it on large screens (600 dp and wider) unless the
/// app is declared a game, which the Velvet manifest does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    /// Landscape, either way up, following the sensor (the default for games).
    #[default]
    SensorLandscape,
    /// Landscape, one way only.
    Landscape,
    /// Portrait, either way up.
    SensorPortrait,
    /// Portrait, one way only.
    Portrait,
    /// Any orientation, following the sensor.
    FullSensor,
    /// Whatever the user's rotation setting allows.
    User,
    /// No preference.
    Unspecified,
}

impl Orientation {
    /// The value of `android:screenOrientation` in a manifest.
    pub fn manifest_value(self) -> &'static str {
        match self {
            Self::SensorLandscape => "sensorLandscape",
            Self::Landscape => "landscape",
            Self::SensorPortrait => "sensorPortrait",
            Self::Portrait => "portrait",
            Self::FullSensor => "fullSensor",
            Self::User => "user",
            Self::Unspecified => "unspecified",
        }
    }

    /// The `ActivityInfo.SCREEN_ORIENTATION_*` constant.
    pub fn android_value(self) -> i32 {
        match self {
            Self::SensorLandscape => 6,
            Self::Landscape => 0,
            Self::SensorPortrait => 7,
            Self::Portrait => 1,
            Self::FullSensor => 10,
            Self::User => 2,
            Self::Unspecified => -1,
        }
    }
}

/// The current safe area of the window (zero on desktop). On Android it is read on the UI
/// thread by [`refresh_safe_area`]; [`crate::Lifecycle`] refreshes it on resume, focus and resize.
pub fn safe_area() -> Insets {
    #[cfg(target_os = "android")]
    {
        crate::android::safe_area()
    }
    #[cfg(not(target_os = "android"))]
    {
        Insets::ZERO
    }
}

/// Reads the window's insets again (Android; a no-op elsewhere).
pub fn refresh_safe_area() {
    #[cfg(target_os = "android")]
    if let Some(app) = crate::android::app() {
        crate::android::refresh_safe_area(&app);
    }
}

/// Immersive full screen: hides the status and navigation bars (Android; a no-op elsewhere).
/// [`crate::Lifecycle::with_immersive`] applies it again whenever Android brings the bars back.
pub fn set_immersive(immersive: bool) {
    #[cfg(target_os = "android")]
    if let Some(app) = crate::android::app() {
        crate::android::set_immersive(&app, immersive);
    }
    #[cfg(not(target_os = "android"))]
    let _ = immersive;
}

/// Keeps the screen from dimming and locking while the game shows (Android; a no-op elsewhere).
pub fn set_keep_screen_on(on: bool) {
    #[cfg(target_os = "android")]
    if let Some(app) = crate::android::app() {
        crate::android::set_keep_screen_on(&app, on);
    }
    #[cfg(not(target_os = "android"))]
    let _ = on;
}

/// Asks for an orientation at run time (Android; a no-op elsewhere).
pub fn set_orientation(orientation: Orientation) {
    #[cfg(target_os = "android")]
    if let Some(app) = crate::android::app() {
        crate::android::set_orientation(&app, orientation);
    }
    #[cfg(not(target_os = "android"))]
    let _ = orientation;
}

/// Shows or hides the soft keyboard (Android; a no-op elsewhere, where a keyboard is assumed).
/// See `docs/platforms/ANDROID.md` for what text input works with it.
pub fn show_keyboard(show: bool) {
    #[cfg(target_os = "android")]
    if let Some(app) = crate::android::app() {
        crate::android::show_keyboard(&app, show);
    }
    #[cfg(not(target_os = "android"))]
    let _ = show;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_rect_shrinks_by_the_insets() {
        let insets = Insets {
            left: 80,
            top: 0,
            right: 40,
            bottom: 20,
        };
        assert_eq!(insets.safe_rect(2400, 1080), (80, 0, 2280, 1060));
        assert_eq!(Insets::ZERO.safe_rect(10, 10), (0, 0, 10, 10));
        let huge = Insets {
            left: 500,
            top: 500,
            right: 500,
            bottom: 500,
        };
        assert_eq!(huge.safe_rect(100, 100), (100, 100, 0, 0));
        assert_eq!(
            insets.max(Insets {
                left: 10,
                top: 30,
                right: 50,
                bottom: 0
            }),
            Insets {
                left: 80,
                top: 30,
                right: 50,
                bottom: 20
            }
        );
    }

    #[test]
    fn orientation_names_match_the_manifest() {
        assert_eq!(
            Orientation::SensorLandscape.manifest_value(),
            "sensorLandscape"
        );
        assert_eq!(Orientation::SensorLandscape.android_value(), 6);
        assert_eq!(Orientation::default(), Orientation::SensorLandscape);
        // Desktop: the calls are harmless no-ops.
        set_immersive(true);
        set_keep_screen_on(true);
        set_orientation(Orientation::Portrait);
        show_keyboard(false);
        refresh_safe_area();
        if !cfg!(target_os = "android") {
            assert!(safe_area().is_zero());
        }
    }
}
