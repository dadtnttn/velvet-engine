//! SVG: a document (often written by code) parsed once and rendered at the pixels it is drawn at, kept until it is
//! drawn at another size. Text inside the SVG is not drawn (draw text with the canvas, which has the game's fonts).

use std::cell::RefCell;
use std::fmt;

/// An SVG document could not be read.
#[derive(Debug)]
pub struct SvgError(String);

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the SVG could not be read: {}", self.0)
    }
}

impl std::error::Error for SvgError {}

/// A parsed SVG document.
pub struct Svg {
    tree: usvg::Tree,
    /// The last rendering, by its pixel size.
    rendered: RefCell<Option<tiny_skia::Pixmap>>,
}

impl fmt::Debug for Svg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Svg").field("size", &self.size()).finish()
    }
}

impl Svg {
    /// Parses an SVG document.
    pub fn parse(text: &str) -> Result<Svg, SvgError> {
        let tree = usvg::Tree::from_str(text, &usvg::Options::default()).map_err(|e| SvgError(e.to_string()))?;
        Ok(Svg { tree, rendered: RefCell::new(None) })
    }

    /// The document's own size (its width and height, or its view box).
    pub fn size(&self) -> (f32, f32) {
        let size = self.tree.size();
        (size.width(), size.height())
    }

    /// The document rendered at a pixel size: the kept rendering when the size is the same.
    pub(crate) fn pixels_for(&self, pw: u32, ph: u32, draw: impl FnOnce(&tiny_skia::Pixmap)) {
        let mut rendered = self.rendered.borrow_mut();
        let fresh = !matches!(&*rendered, Some(p) if p.width() == pw && p.height() == ph);
        if fresh {
            let Some(mut pixmap) = tiny_skia::Pixmap::new(pw.max(1), ph.max(1)) else { return };
            let (w, h) = self.size();
            let transform = tiny_skia::Transform::from_scale(pw as f32 / w, ph as f32 / h);
            resvg::render(&self.tree, transform, &mut pixmap.as_mut());
            *rendered = Some(pixmap);
        }
        if let Some(p) = &*rendered {
            draw(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_renders_at_the_asked_size() {
        let svg = Svg::parse(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="#ff0000"/></svg>"##).unwrap();
        assert_eq!(svg.size(), (20.0, 10.0));
        let mut seen = None;
        svg.pixels_for(40, 20, |p| seen = Some((p.width(), p.height(), p.pixel(20, 10).map(|c| c.red()))));
        assert_eq!(seen, Some((40, 20, Some(255))));
        assert!(Svg::parse("<not svg").is_err());
    }
}
