//! Where the game's 16:9 (or any aspect) picture sits in the window: the largest centred rectangle of its aspect, with
//! bars around it, and the exact way back from the window's pixels to the game's logical coordinates. It follows the
//! same rules as velvet-render's `SoftwareViewport` (kept here so a 2D game does not pull in the 3D renderer).

/// The game's picture in the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// The picture's left and top edges and its size, in window pixels.
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// The game's logical size.
    pub logical_width: f32,
    pub logical_height: f32,
}

impl Viewport {
    /// The largest centred rectangle of the logical aspect inside a window, or None for an empty window.
    pub fn letterbox(
        window_width: u32,
        window_height: u32,
        logical_width: f32,
        logical_height: f32,
    ) -> Option<Viewport> {
        if window_width == 0 || window_height == 0 || logical_width <= 0.0 || logical_height <= 0.0
        {
            return None;
        }
        let scale =
            (window_width as f32 / logical_width).min(window_height as f32 / logical_height);
        let width = ((logical_width * scale).round() as u32).clamp(1, window_width);
        let height = ((logical_height * scale).round() as u32).clamp(1, window_height);
        Some(Viewport {
            x: (window_width - width) / 2,
            y: (window_height - height) / 2,
            width,
            height,
            logical_width,
            logical_height,
        })
    }

    /// A window pixel in logical coordinates, or None in the bars.
    pub fn to_logical(&self, px: f64, py: f64) -> Option<(f32, f32)> {
        let lx = (px - self.x as f64) * self.logical_width as f64 / self.width as f64;
        let ly = (py - self.y as f64) * self.logical_height as f64 / self.height as f64;
        let inside = lx >= 0.0
            && ly >= 0.0
            && lx < self.logical_width as f64
            && ly < self.logical_height as f64;
        inside.then_some((lx as f32, ly as f32))
    }

    /// A window pixel in logical coordinates, held to the picture's edges (for a release in the bars).
    pub fn to_logical_clamped(&self, px: f64, py: f64) -> (f32, f32) {
        let lx = (px - self.x as f64) * self.logical_width as f64 / self.width as f64;
        let ly = (py - self.y as f64) * self.logical_height as f64 / self.height as f64;
        (
            lx.clamp(0.0, self.logical_width as f64 - 0.001) as f32,
            ly.clamp(0.0, self.logical_height as f64 - 0.001) as f32,
        )
    }

    /// A logical point in window pixels.
    pub fn to_window(&self, lx: f32, ly: f32) -> (f32, f32) {
        (
            self.x as f32 + lx * self.width as f32 / self.logical_width,
            self.y as f32 + ly * self.height as f32 / self.logical_height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_window_gets_bars_left_and_right() {
        let v = Viewport::letterbox(2560, 1080, 1920.0, 1080.0).unwrap();
        assert_eq!((v.x, v.y, v.width, v.height), (320, 0, 1920, 1080));
        assert_eq!(v.to_logical(100.0, 500.0), None, "in the bar");
        assert_eq!(v.to_logical(320.0, 0.0), Some((0.0, 0.0)));
    }

    #[test]
    fn a_tall_window_gets_bars_above_and_below() {
        let v = Viewport::letterbox(1280, 1024, 1920.0, 1080.0).unwrap();
        assert_eq!((v.x, v.y, v.width, v.height), (0, 152, 1280, 720));
    }

    #[test]
    fn the_way_back_is_exact_at_any_size() {
        for (w, h) in [
            (1280, 720),
            (1600, 900),
            (2560, 1440),
            (1366, 768),
            (3440, 1440),
        ] {
            let v = Viewport::letterbox(w, h, 1920.0, 1080.0).unwrap();
            for (lx, ly) in [(0.0, 0.0), (960.0, 540.0), (1919.0, 1079.0), (123.4, 987.6)] {
                let (px, py) = v.to_window(lx, ly);
                let (bx, by) = v.to_logical(px as f64, py as f64).unwrap();
                assert!(
                    (bx - lx).abs() < 0.01 && (by - ly).abs() < 0.01,
                    "{w}×{h}: {lx},{ly} → {bx},{by}"
                );
            }
        }
    }

    #[test]
    fn a_release_in_the_bars_is_held_to_the_edge() {
        let v = Viewport::letterbox(2560, 1080, 1920.0, 1080.0).unwrap();
        let (x, y) = v.to_logical_clamped(5.0, 540.0);
        assert_eq!(x, 0.0);
        assert!((y - 540.0).abs() < 0.01);
        assert!(Viewport::letterbox(0, 100, 1920.0, 1080.0).is_none());
    }
}
