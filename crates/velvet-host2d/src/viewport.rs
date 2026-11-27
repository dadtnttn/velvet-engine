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
        Self::letterbox_in(
            0,
            0,
            window_width,
            window_height,
            logical_width,
            logical_height,
        )
    }

    /// The largest centred rectangle of the logical aspect inside the sub-rectangle
    /// `(area_x, area_y, area_width, area_height)` (such as the screen's safe area outside display
    /// cutouts), or None for an empty area.
    pub fn letterbox_in(
        area_x: u32,
        area_y: u32,
        area_width: u32,
        area_height: u32,
        logical_width: f32,
        logical_height: f32,
    ) -> Option<Viewport> {
        if area_width == 0 || area_height == 0 || logical_width <= 0.0 || logical_height <= 0.0 {
            return None;
        }
        let scale = (area_width as f32 / logical_width).min(area_height as f32 / logical_height);
        let width = ((logical_width * scale).round() as u32).clamp(1, area_width);
        let height = ((logical_height * scale).round() as u32).clamp(1, area_height);
        Some(Viewport {
            x: area_x + (area_width - width) / 2,
            y: area_y + (area_height - height) / 2,
            width,
            height,
            logical_width,
            logical_height,
        })
    }

    /// The whole window, if its shape is within `tolerance` of the picture's (0.04 is 4 %): a maximised window that
    /// is nearly the right shape gets no thin bars, and a stretch that small can't be seen. Otherwise as it was.
    pub fn filling(self, window_width: u32, window_height: u32, tolerance: f32) -> Viewport {
        self.filling_in(0, 0, window_width, window_height, tolerance)
    }

    /// Like [`Self::filling`], within the given area.
    pub fn filling_in(
        self,
        area_x: u32,
        area_y: u32,
        area_width: u32,
        area_height: u32,
        tolerance: f32,
    ) -> Viewport {
        let window = area_width as f32 / area_height.max(1) as f32;
        let picture = self.logical_width / self.logical_height;
        if tolerance > 0.0 && (window / picture - 1.0).abs() <= tolerance {
            Viewport {
                x: area_x,
                y: area_y,
                width: area_width,
                height: area_height,
                ..self
            }
        } else {
            self
        }
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
    fn a_window_nearly_the_right_shape_is_filled() {
        let fit = |w, h, t| {
            Viewport::letterbox(w, h, 1920.0, 1080.0)
                .unwrap()
                .filling(w, h, t)
        };
        let near = fit(2000, 1154, 0.04);
        assert_eq!(
            (near.x, near.y, near.width, near.height),
            (0, 0, 2000, 1154)
        );
        // A real 16:10 screen and a game that asked for nothing keep the picture's shape.
        assert!(fit(2560, 1600, 0.04).y > 0);
        assert!(fit(2000, 1154, 0.0).y > 0);
    }

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
