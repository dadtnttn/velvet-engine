//! Nine-patch layout math (source UVs and destination rects).

use velvet_math::{Rect, Vec2};

/// Nine-patch margins in pixels (source texture space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NinePatchMargins {
    /// Left border width.
    pub left: f32,
    /// Top border height.
    pub top: f32,
    /// Right border width.
    pub right: f32,
    /// Bottom border height.
    pub bottom: f32,
}

impl NinePatchMargins {
    /// Uniform margin on all sides.
    pub fn uniform(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    /// Create.
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: left.max(0.0),
            top: top.max(0.0),
            right: right.max(0.0),
            bottom: bottom.max(0.0),
        }
    }
}

/// One of the nine patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NinePatchCell {
    /// Top-left corner.
    TopLeft,
    /// Top edge.
    Top,
    /// Top-right corner.
    TopRight,
    /// Left edge.
    Left,
    /// Center (stretch).
    Center,
    /// Right edge.
    Right,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom edge.
    Bottom,
    /// Bottom-right corner.
    BottomRight,
}

/// Source UV (or pixel) rect and destination rect for one cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NinePatchQuad {
    /// Cell kind.
    pub cell: NinePatchCell,
    /// Source rectangle in texture pixels.
    pub src: Rect,
    /// Destination rectangle in UI/screen space.
    pub dst: Rect,
}

/// Compute nine destination/source quads for drawing a nine-patch.
///
/// `src_size` is the source texture (or region) size in pixels.
/// `dst` is the destination outer rect.
pub fn layout_nine_patch(
    src_size: Vec2,
    margins: NinePatchMargins,
    dst: Rect,
) -> [NinePatchQuad; 9] {
    let sw = src_size.x.max(1.0);
    let sh = src_size.y.max(1.0);
    let ml = margins.left.min(sw * 0.5);
    let mr = margins.right.min(sw * 0.5);
    let mt = margins.top.min(sh * 0.5);
    let mb = margins.bottom.min(sh * 0.5);

    // Source splits
    let sx0 = 0.0;
    let sx1 = ml;
    let sx2 = sw - mr;
    let sx3 = sw;
    let sy0 = 0.0;
    let sy1 = mt;
    let sy2 = sh - mb;
    let sy3 = sh;

    // Dest: corners keep source pixel size; edges/center stretch.
    let dw = dst.width().max(0.0);
    let dh = dst.height().max(0.0);
    // Shrink margins if dest smaller than borders.
    let dl = ml.min(dw * 0.5);
    let dr = mr.min(dw * 0.5);
    let dt = mt.min(dh * 0.5);
    let db = mb.min(dh * 0.5);

    let dx0 = dst.min.x;
    let dx1 = dst.min.x + dl;
    let dx2 = dst.max.x - dr;
    let dx3 = dst.max.x;
    let dy0 = dst.min.y;
    let dy1 = dst.min.y + dt;
    let dy2 = dst.max.y - db;
    let dy3 = dst.max.y;

    let cell = |cell: NinePatchCell,
                sx0: f32,
                sy0: f32,
                sx1: f32,
                sy1: f32,
                dx0: f32,
                dy0: f32,
                dx1: f32,
                dy1: f32| {
        NinePatchQuad {
            cell,
            src: Rect::from_min_max(Vec2::new(sx0, sy0), Vec2::new(sx1, sy1)),
            dst: Rect::from_min_max(Vec2::new(dx0, dy0), Vec2::new(dx1, dy1)),
        }
    };

    [
        cell(
            NinePatchCell::TopLeft,
            sx0,
            sy0,
            sx1,
            sy1,
            dx0,
            dy0,
            dx1,
            dy1,
        ),
        cell(NinePatchCell::Top, sx1, sy0, sx2, sy1, dx1, dy0, dx2, dy1),
        cell(
            NinePatchCell::TopRight,
            sx2,
            sy0,
            sx3,
            sy1,
            dx2,
            dy0,
            dx3,
            dy1,
        ),
        cell(NinePatchCell::Left, sx0, sy1, sx1, sy2, dx0, dy1, dx1, dy2),
        cell(
            NinePatchCell::Center,
            sx1,
            sy1,
            sx2,
            sy2,
            dx1,
            dy1,
            dx2,
            dy2,
        ),
        cell(NinePatchCell::Right, sx2, sy1, sx3, sy2, dx2, dy1, dx3, dy2),
        cell(
            NinePatchCell::BottomLeft,
            sx0,
            sy2,
            sx1,
            sy3,
            dx0,
            dy2,
            dx1,
            dy3,
        ),
        cell(
            NinePatchCell::Bottom,
            sx1,
            sy2,
            sx2,
            sy3,
            dx1,
            dy2,
            dx2,
            dy3,
        ),
        cell(
            NinePatchCell::BottomRight,
            sx2,
            sy2,
            sx3,
            sy3,
            dx2,
            dy2,
            dx3,
            dy3,
        ),
    ]
}

/// Content-safe inner rect after nine-patch margins (destination space).
pub fn content_rect(dst: Rect, margins: NinePatchMargins) -> Rect {
    Rect::from_min_max(
        Vec2::new(dst.min.x + margins.left, dst.min.y + margins.top),
        Vec2::new(dst.max.x - margins.right, dst.max.y - margins.bottom),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_keep_size() {
        let dst = Rect::from_pos_size(Vec2::new(10.0, 20.0), Vec2::new(200.0, 100.0));
        let m = NinePatchMargins::uniform(8.0);
        let quads = layout_nine_patch(Vec2::new(32.0, 32.0), m, dst);
        let tl = quads[0];
        assert!((tl.dst.width() - 8.0).abs() < 1e-4);
        assert!((tl.dst.height() - 8.0).abs() < 1e-4);
        let center = quads[4];
        assert!(center.dst.width() > 100.0);
    }

    #[test]
    fn content_rect_inset() {
        let dst = Rect::from_pos_size(Vec2::ZERO, Vec2::new(100.0, 50.0));
        let inner = content_rect(dst, NinePatchMargins::new(10.0, 5.0, 10.0, 5.0));
        assert!((inner.width() - 80.0).abs() < 1e-4);
        assert!((inner.height() - 40.0).abs() < 1e-4);
    }
}
