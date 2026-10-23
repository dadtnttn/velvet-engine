//! Signed distance fields for sprite bodies (a port of `sdf.js`). Every field
//! returns `(d, r)`: d < 0 inside, and r, the local thickness used to "inflate"
//! the silhouette into a rounded surface. Smooth unions fuse limbs into
//! shoulders and hands into wrists, so a body shades and outlines as one form.
use std::rc::Rc;

use crate::{clamp, P2};

pub type Field = Rc<dyn Fn(f64, f64) -> (f64, f64)>;

pub fn circle(cx: f64, cy: f64, r: f64, t: f64) -> Field {
    Rc::new(move |x, y| ((x - cx).hypot(y - cy) - r, t))
}

pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Field {
    let m = rx.min(ry);
    Rc::new(move |x, y| ((((x - cx) / rx).hypot((y - cy) / ry) - 1.0) * m, m))
}

/// Tapered capsule from a (radius ra) to b (radius rb); `t` overrides the
/// thickness (default: the local radius).
pub fn capsule(ax: f64, ay: f64, bx: f64, by: f64, ra: f64, rb: f64, t: Option<f64>) -> Field {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = {
        let l = dx * dx + dy * dy;
        if l == 0.0 {
            1e-6
        } else {
            l
        }
    };
    Rc::new(move |x, y| {
        let s = clamp(((x - ax) * dx + (y - ay) * dy) / l2, 0.0, 1.0);
        let r = ra + (rb - ra) * s;
        ((x - ax - dx * s).hypot(y - ay - dy * s) - r, t.unwrap_or(r))
    })
}

/// Polygon with exact edge distance; `t` is its inflation thickness.
pub fn poly(pts: Vec<P2>, t: f64) -> Field {
    Rc::new(move |x, y| {
        let mut d = f64::INFINITY;
        let mut inside = false;
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let [ax, ay] = pts[j];
            let [bx, by] = pts[i];
            let (ex, ey) = (bx - ax, by - ay);
            let (wx, wy) = (x - ax, y - ay);
            let denom = {
                let v = ex * ex + ey * ey;
                if v == 0.0 {
                    1e-6
                } else {
                    v
                }
            };
            let s = clamp((wx * ex + wy * ey) / denom, 0.0, 1.0);
            let (qx, qy) = (wx - ex * s, wy - ey * s);
            d = d.min(qx * qx + qy * qy);
            if (ay > y) != (by > y) && x < ((bx - ax) * (y - ay)) / (by - ay) + ax {
                inside = !inside;
            }
            j = i;
        }
        ((if inside { -1.0 } else { 1.0 }) * d.sqrt(), t)
    })
}

/// Smooth union with blend radius k (thickness blends with it).
pub fn smin(k: f64, fields: Vec<Field>) -> Field {
    Rc::new(move |x, y| {
        let (mut d, mut r) = fields[0](x, y);
        for f in &fields[1..] {
            let (d2, r2) = f(x, y);
            let h = clamp(0.5 + (0.5 * (d2 - d)) / k, 0.0, 1.0);
            d = d2 + (d - d2) * h - k * h * (1.0 - h);
            r = r2 + (r - r2) * h;
        }
        (d, r)
    })
}

pub fn union(fields: Vec<Field>) -> Field {
    Rc::new(move |x, y| {
        let mut best: Option<(f64, f64)> = None;
        for f in &fields {
            let v = f(x, y);
            if best.is_none_or(|b| v.0 < b.0) {
                best = Some(v);
            }
        }
        best.expect("union of at least one field")
    })
}
