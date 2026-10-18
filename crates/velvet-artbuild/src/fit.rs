//! Fits a source picture to the box of its art id, at the output size. Three ways are offered:
//! `cover` crops the picture to the box's proportions and keeps the part that the focus names,
//! `contain` shows the whole picture inside the box with transparent bands, and `stretch` changes
//! the proportions to fit exactly. Every resize uses the Lanczos3 filter, which keeps detail.

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use serde::Serialize;

use crate::manifest::Kind;

/// How a source picture fills its box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Crop to the box's proportions, keeping the part that the focus names. The default.
    Cover,
    /// Show the whole picture, centred, inside the box. The bands that are left are transparent.
    Contain,
    /// Change the proportions to fill the box exactly. The picture is distorted.
    Stretch,
}

impl Fit {
    /// The name of the mode, as the build config writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Fit::Cover => "cover",
            Fit::Contain => "contain",
            Fit::Stretch => "stretch",
        }
    }

    /// The mode with this name, or None for any other name.
    pub fn from_name(name: &str) -> Option<Fit> {
        [Fit::Cover, Fit::Contain, Fit::Stretch]
            .into_iter()
            .find(|fit| fit.as_str() == name)
    }
}

/// The focus of a cover crop when the config does not set one. Portraits and full bodies keep the
/// top of the picture, so that heads are not cut; the other kinds keep the middle.
pub fn default_focus(kind: Kind) -> [f64; 2] {
    match kind {
        Kind::Portrait | Kind::Fullbody => [0.5, 0.0],
        Kind::Card | Kind::Icon | Kind::Background | Kind::Prop => [0.5, 0.5],
    }
}

/// Renders a source picture into a box of `width` by `height` output pixels.
///
/// `focus` is the `[x, y]` point, as fractions, that a `cover` crop keeps in place of the middle: `[0, 0]`
/// keeps the top left, `[1, 1]` the bottom right. It is ignored by the other modes.
pub fn render(source: &RgbaImage, fit: Fit, focus: [f64; 2], width: u32, height: u32) -> RgbaImage {
    match fit {
        Fit::Stretch => imageops::resize(source, width, height, FilterType::Lanczos3),
        Fit::Cover => {
            let (x, y, window_width, window_height) =
                cover_window(source.width(), source.height(), width, height, focus);
            let window = imageops::crop_imm(source, x, y, window_width, window_height).to_image();
            imageops::resize(&window, width, height, FilterType::Lanczos3)
        }
        Fit::Contain => {
            let (picture_width, picture_height) =
                contain_size(source.width(), source.height(), width, height);
            let picture =
                imageops::resize(source, picture_width, picture_height, FilterType::Lanczos3);
            let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
            let x = i64::from((width - picture_width) / 2);
            let y = i64::from((height - picture_height) / 2);
            imageops::replace(&mut canvas, &picture, x, y);
            canvas
        }
    }
}

/// The part of a source of `source_width` by `source_height` that a cover crop keeps, for a box of
/// `box_width` by `box_height`. It is the largest window with the box's proportions, placed so that
/// the focus point stays at the same fraction of the slack on each axis. Returns `(x, y, width, height)`.
pub(crate) fn cover_window(
    source_width: u32,
    source_height: u32,
    box_width: u32,
    box_height: u32,
    focus: [f64; 2],
) -> (u32, u32, u32, u32) {
    let wider = u64::from(source_width) * u64::from(box_height)
        > u64::from(source_height) * u64::from(box_width);
    let (window_width, window_height) = if wider {
        // The picture is wider than the box: keep all of its height and cut the sides.
        let width =
            (f64::from(source_height) * f64::from(box_width) / f64::from(box_height)).round();
        (
            width.clamp(1.0, f64::from(source_width)) as u32,
            source_height,
        )
    } else {
        // The picture is taller than the box: keep all of its width and cut the top and bottom.
        let height =
            (f64::from(source_width) * f64::from(box_height) / f64::from(box_width)).round();
        (
            source_width,
            height.clamp(1.0, f64::from(source_height)) as u32,
        )
    };
    let slack_x = f64::from(source_width - window_width);
    let slack_y = f64::from(source_height - window_height);
    let x = (slack_x * focus[0].clamp(0.0, 1.0)).round() as u32;
    let y = (slack_y * focus[1].clamp(0.0, 1.0)).round() as u32;
    (
        x.min(source_width - window_width),
        y.min(source_height - window_height),
        window_width,
        window_height,
    )
}

/// The size of a source of `source_width` by `source_height` when it is scaled to fit inside a box
/// of `box_width` by `box_height` without changing its proportions. Neither side is less than 1.
pub(crate) fn contain_size(
    source_width: u32,
    source_height: u32,
    box_width: u32,
    box_height: u32,
) -> (u32, u32) {
    let scale = (f64::from(box_width) / f64::from(source_width))
        .min(f64::from(box_height) / f64::from(source_height));
    let width = (f64::from(source_width) * scale)
        .round()
        .clamp(1.0, f64::from(box_width));
    let height = (f64::from(source_height) * scale)
        .round()
        .clamp(1.0, f64::from(box_height));
    (width as u32, height as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture whose left half is red and right half is blue, for checking where a crop lands.
    fn halves(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        })
    }

    #[test]
    fn cover_keeps_the_middle_by_default_and_the_focus_moves_the_window() {
        assert_eq!(
            cover_window(200, 100, 100, 100, [0.5, 0.5]),
            (50, 0, 100, 100)
        );
        assert_eq!(
            cover_window(200, 100, 100, 100, [0.0, 0.5]),
            (0, 0, 100, 100)
        );
        assert_eq!(
            cover_window(200, 100, 100, 100, [1.0, 0.5]),
            (100, 0, 100, 100)
        );
    }

    #[test]
    fn a_tall_picture_keeps_its_top_for_a_portrait_focus() {
        assert_eq!(
            cover_window(100, 200, 100, 100, [0.5, 0.0]),
            (0, 0, 100, 100)
        );
        assert_eq!(
            cover_window(100, 200, 100, 100, [0.5, 1.0]),
            (0, 100, 100, 100)
        );
    }

    #[test]
    fn cover_renders_exactly_the_box_and_crops_the_chosen_side() {
        let source = halves(200, 100);
        let left = render(&source, Fit::Cover, [0.0, 0.5], 40, 40);
        assert_eq!(left.dimensions(), (40, 40));
        assert!(left.pixels().all(|p| p.0[0] == 255 && p.0[2] == 0));
        let right = render(&source, Fit::Cover, [1.0, 0.5], 40, 40);
        assert!(right.pixels().all(|p| p.0[2] == 255 && p.0[0] == 0));
    }

    #[test]
    fn contain_centres_the_picture_and_leaves_transparent_bands() {
        assert_eq!(contain_size(200, 100, 100, 100), (100, 50));
        let source = RgbaImage::from_pixel(200, 100, Rgba([10, 20, 30, 255]));
        let canvas = render(&source, Fit::Contain, [0.5, 0.5], 100, 100);
        assert_eq!(canvas.dimensions(), (100, 100));
        assert_eq!(canvas.get_pixel(50, 0).0[3], 0);
        assert_eq!(canvas.get_pixel(50, 99).0[3], 0);
        assert_eq!(canvas.get_pixel(50, 50).0[3], 255);
    }

    #[test]
    fn stretch_fills_the_box_whatever_the_proportions() {
        let source = halves(300, 60);
        let stretched = render(&source, Fit::Stretch, [0.5, 0.5], 64, 64);
        assert_eq!(stretched.dimensions(), (64, 64));
        assert_eq!(stretched.get_pixel(0, 32).0[0], 255);
        assert_eq!(stretched.get_pixel(63, 32).0[2], 255);
    }

    #[test]
    fn default_focus_keeps_the_top_of_people() {
        assert_eq!(default_focus(Kind::Portrait), [0.5, 0.0]);
        assert_eq!(default_focus(Kind::Fullbody), [0.5, 0.0]);
        assert_eq!(default_focus(Kind::Background), [0.5, 0.5]);
    }

    #[test]
    fn fit_names_round_trip() {
        for fit in [Fit::Cover, Fit::Contain, Fit::Stretch] {
            assert_eq!(Fit::from_name(fit.as_str()), Some(fit));
        }
        assert_eq!(Fit::from_name("fill"), None);
    }
}
