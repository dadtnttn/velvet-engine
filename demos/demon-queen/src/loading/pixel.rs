//! Tiny pixel-art toolkit for the loading screens (a port of `pixel.js` in the
//! Three.js cave project). Characters and props are built from shaded
//! primitives (ellipses, tapered capsules, polygons, signed-distance fields)
//! into a material buffer, then lit with banded ramps, cast shadows, rim light
//! and selective outlines, so every frame reads like hand-shaded pixel art.
//!
//! The port keeps the JavaScript's number semantics where they show up in the
//! image: `f64` maths, `f32` storage for the normal/tone buffers, `Math.round`
//! (ties toward +infinity), truncating `|0`, and `Uint8ClampedArray`'s
//! round-half-to-even when writing channels.
use std::collections::HashMap;
use std::rc::Rc;

pub const W: usize = 480;
pub const H: usize = 270;

pub type Rgb = [f64; 3];
pub type P2 = [f64; 2];

/// `Math.round`: halves go toward +infinity.
pub fn jr(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// `x | 0` for the small values used here.
pub fn trunc(v: f64) -> i32 {
    v as i32
}

pub fn hex(s: &str) -> Rgb {
    let n = u32::from_str_radix(s.trim_start_matches('#'), 16).expect("hex colour");
    [
        ((n >> 16) & 255) as f64,
        ((n >> 8) & 255) as f64,
        (n & 255) as f64,
    ]
}

pub fn ramp(hexes: &[&str]) -> Vec<Rgb> {
    hexes.iter().map(|h| hex(h)).collect()
}

pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

pub fn shade(c: Rgb, k: f64) -> Rgb {
    [c[0] * k, c[1] * k, c[2] * k]
}

pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

const BAYER: [i32; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

pub fn bayer(x: i32, y: i32) -> f64 {
    (BAYER[(((y & 3) << 2) | (x & 3)) as usize] as f64 + 0.5) / 16.0
}

/// Deterministic hash noise in [0, 1).
pub fn hash(x: f64, y: f64, seed: f64) -> f64 {
    let (x, y, seed) = (x as i32, y as i32, seed as i32);
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(2246822519u32 as i32));
    h = (h ^ (((h as u32) >> 13) as i32)).wrapping_mul(1274126177);
    (((h ^ (((h as u32) >> 16) as i32)) as u32) as f64) / 4294967296.0
}

/// Pick a ramp colour for a continuous level with ordered dithering.
pub fn pick(ramp: &[Rgb], level: f64, x: i32, y: i32, dither: f64) -> Rgb {
    let n = ramp.len();
    let l = level * (n as f64 - 1.0) + (bayer(x, y) - 0.5) * dither;
    ramp[clamp(jr(l), 0.0, (n - 1) as f64) as usize]
}

/// Channel write into a `Uint8ClampedArray`: clamp, then round half to even.
fn to_u8(v: f64) -> u8 {
    if v.is_nan() {
        0
    } else {
        v.clamp(0.0, 255.0).round_ties_even() as u8
    }
}

#[derive(Clone)]
pub struct Surface {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
}

impl Surface {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            data: vec![0; w * h * 4],
        }
    }

    pub fn from_rgba(w: usize, h: usize, data: Vec<u8>) -> Self {
        assert_eq!(data.len(), w * h * 4);
        Self { w, h, data }
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h
    }

    pub fn set(&mut self, x: f64, y: f64, c: Rgb, a: f64) {
        self.set_i(trunc(x), trunc(y), c, a);
    }

    pub fn set_i(&mut self, x: i32, y: i32, c: Rgb, a: f64) {
        if !self.inside(x, y) {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        let d = &mut self.data;
        if a >= 1.0 {
            d[i] = to_u8(c[0]);
            d[i + 1] = to_u8(c[1]);
            d[i + 2] = to_u8(c[2]);
            d[i + 3] = 255;
            return;
        }
        for k in 0..3 {
            let old = d[i + k] as f64;
            d[i + k] = to_u8(old + (c[k] - old) * a);
        }
        d[i + 3] = d[i + 3].max(to_u8(a * 255.0));
    }

    pub fn add(&mut self, x: f64, y: f64, c: Rgb, k: f64) {
        let (x, y) = (trunc(x), trunc(y));
        if !self.inside(x, y) {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        for n in 0..3 {
            self.data[i + n] = to_u8(self.data[i + n] as f64 + c[n] * k);
        }
    }

    /// Copy every opaque pixel of `src` with its top-left at (ox, oy).
    pub fn blit(&mut self, src: &Surface, ox: f64, oy: f64, flip: bool) {
        let (ox, oy) = (trunc(ox), trunc(oy));
        for y in 0..src.h as i32 {
            let ty = oy + y;
            if ty < 0 || ty >= self.h as i32 {
                continue;
            }
            for x in 0..src.w as i32 {
                let i = (y as usize * src.w + x as usize) * 4;
                let a = src.data[i + 3];
                if a == 0 {
                    continue;
                }
                let tx = if flip {
                    ox + src.w as i32 - 1 - x
                } else {
                    ox + x
                };
                if tx < 0 || tx >= self.w as i32 {
                    continue;
                }
                let j = (ty as usize * self.w + tx as usize) * 4;
                if a == 255 {
                    self.data[j..j + 4].copy_from_slice(&src.data[i..i + 4]);
                    continue;
                }
                let k = a as f64 / 255.0;
                for n in 0..3 {
                    let old = self.data[j + n] as f64;
                    self.data[j + n] = to_u8(old + (src.data[i + n] as f64 - old) * k);
                }
                self.data[j + 3] = self.data[j + 3].max(a);
            }
        }
    }

    pub fn copy_from(&mut self, src: &Surface) {
        self.data.copy_from_slice(&src.data);
    }
}

/// Banded, dithered radial light added onto a surface (candles, moons).
pub fn glow(s: &mut Surface, cx: f64, cy: f64, radius: f64, color: Rgb, strength: f64, bands: f64) {
    let r2 = radius * radius;
    for y in (cy - radius).floor() as i32..=(cy + radius).floor() as i32 {
        for x in (cx - radius).floor() as i32..=(cx + radius).floor() as i32 {
            let d2 = (x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2);
            if d2 > r2 {
                continue;
            }
            let f = 1.0 - d2.sqrt() / radius;
            let q = (f * f * bands + bayer(x, y) * 0.9).floor() / bands;
            if q > 0.0 {
                s.add(x as f64, y as f64, color, q * strength);
            }
        }
    }
}

const DX: [i32; 4] = [1, -1, 0, 0];
const DY: [i32; 4] = [0, 0, 1, -1];

fn norm3(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let l = if l == 0.0 { 1.0 } else { l };
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Material: a dark→light ramp plus outline colours and shading options.
pub struct Material {
    pub ramp: Vec<Rgb>,
    pub inner: Rgb,
    pub line: Rgb,
    pub dither: f64,
    pub contrast: f64,
    pub bias: f64,
    pub rim: bool,
    pub outline: bool,
}

#[derive(Default, Clone)]
pub struct MatOpts {
    dither: Option<f64>,
    line: Option<Rgb>,
    inner: Option<Rgb>,
    outline: Option<bool>,
    rim: Option<bool>,
    contrast: Option<f64>,
}

impl MatOpts {
    pub fn dither(mut self, v: f64) -> Self {
        self.dither = Some(v);
        self
    }
    pub fn line(mut self, c: [i32; 3]) -> Self {
        self.line = Some([c[0] as f64, c[1] as f64, c[2] as f64]);
        self
    }
    pub fn inner(mut self, c: [i32; 3]) -> Self {
        self.inner = Some([c[0] as f64, c[1] as f64, c[2] as f64]);
        self
    }
    pub fn no_outline(mut self) -> Self {
        self.outline = Some(false);
        self
    }
    pub fn contrast(mut self, v: f64) -> Self {
        self.contrast = Some(v);
        self
    }
}

/// Material helper: ramp dark→light plus optional line colours.
pub fn material(r: Vec<Rgb>, o: MatOpts) -> Rc<Material> {
    Rc::new(Material {
        inner: o.inner.unwrap_or_else(|| shade(r[0], 0.8)),
        line: o.line.unwrap_or_else(|| shade(r[0], 0.5)),
        dither: o.dither.unwrap_or(0.35),
        contrast: o.contrast.unwrap_or(1.0),
        bias: 0.0,
        rim: o.rim.unwrap_or(true),
        outline: o.outline.unwrap_or(true),
        ramp: r,
    })
}

/// A material given as a bare ramp (no `material()` helper): its inner line
/// is its darkest colour, and it draws no outline unless asked.
pub fn raw_material(r: Vec<Rgb>, dither: f64, outline: bool, rim: bool) -> Rc<Material> {
    Rc::new(Material {
        inner: r[0],
        line: shade(r[0], 0.55),
        dither,
        contrast: 1.0,
        bias: 0.0,
        rim,
        outline,
        ramp: r,
    })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Shade {
    Flat,
    Cyl,
    Dome,
    VCyl,
}

/// Per-shape options (the JavaScript `o` object).
#[derive(Default)]
pub struct O<'a> {
    pub line: Option<bool>,
    pub line_same: bool,
    pub cast: i32,
    pub joins: Option<Vec<[f64; 3]>>,
    pub rim_over: bool,
    pub mask: Option<Vec<Rc<Material>>>,
    pub under: bool,
    pub where_: Option<&'a dyn Fn(i32, i32) -> bool>,
    pub round: Option<f64>,
    pub tilt: Option<[f64; 2]>,
    pub tone: f64,
    pub flat: bool,
    pub normal: Option<[f64; 3]>,
    pub steps: Option<usize>,
    pub flatten: f64,
    pub shade: Option<Shade>,
    pub curve: Option<f64>,
    pub soft: Option<f64>,
}

impl<'a> O<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cast(mut self, v: i32) -> Self {
        self.cast = v;
        self
    }
    pub fn line(mut self, v: bool) -> Self {
        self.line = Some(v);
        self
    }
    pub fn line_same(mut self) -> Self {
        self.line_same = true;
        self
    }
    pub fn joins(mut self, j: Vec<[f64; 3]>) -> Self {
        self.joins = Some(j);
        self
    }
    pub fn rim_over(mut self) -> Self {
        self.rim_over = true;
        self
    }
    pub fn round(mut self, v: f64) -> Self {
        self.round = Some(v);
        self
    }
    pub fn tone(mut self, v: f64) -> Self {
        self.tone = v;
        self
    }
    pub fn flat(mut self) -> Self {
        self.flat = true;
        self
    }
    pub fn steps(mut self, v: usize) -> Self {
        self.steps = Some(v);
        self
    }
    pub fn flatten(mut self, v: f64) -> Self {
        self.flatten = v;
        self
    }
    pub fn shade(mut self, v: Shade) -> Self {
        self.shade = Some(v);
        self
    }
    pub fn curve(mut self, v: f64) -> Self {
        self.curve = Some(v);
        self
    }
    pub fn where_(mut self, f: &'a dyn Fn(i32, i32) -> bool) -> Self {
        self.where_ = Some(f);
        self
    }
}

struct ShapeInfo {
    mat: usize,
    line: bool,
    line_same: bool,
    cast: i32,
    joins: Option<Vec<[f64; 3]>>,
    rim_over: bool,
}

struct Texture {
    mat: usize,
    z: i32,
    f: Box<dyn Fn(i32, i32) -> f64>,
}

pub struct RimLight {
    pub dir: [i32; 2],
    pub color: Rgb,
    pub k: f64,
}

#[derive(Default)]
pub struct Recolor<'a> {
    pub only: Option<Vec<Rc<Material>>>,
    pub shapes: Option<Vec<i16>>,
    pub box_: Option<[f64; 4]>,
    pub tone: f64,
    _p: std::marker::PhantomData<&'a ()>,
}

impl<'a> Recolor<'a> {
    pub fn shapes(ids: &[i16]) -> Self {
        Self {
            shapes: Some(ids.to_vec()),
            ..Default::default()
        }
    }
}

/// A figure is painted with shaded shapes in draw order (later = in front).
pub struct Figure {
    pub w: usize,
    pub h: usize,
    mat: Vec<i16>,
    shape: Vec<i16>,
    nx: Vec<f32>,
    ny: Vec<f32>,
    nz: Vec<f32>,
    tone: Vec<f32>,
    mats: Vec<Rc<Material>>,
    shapes: Vec<ShapeInfo>,
    light: [f64; 3],
    rim: Option<RimLight>,
    details: Vec<(i32, i32, Rgb, i32)>,
    textures: Vec<Texture>,
}

impl Figure {
    pub fn new(w: usize, h: usize, light: [f64; 3], rim: Option<RimLight>) -> Self {
        let n = w * h;
        Self {
            w,
            h,
            mat: vec![-1; n],
            shape: vec![-1; n],
            nx: vec![0.0; n],
            ny: vec![0.0; n],
            nz: vec![0.0; n],
            tone: vec![0.0; n],
            mats: Vec::new(),
            shapes: Vec::new(),
            light: norm3(light),
            rim,
            details: Vec::new(),
            textures: Vec::new(),
        }
    }

    fn mat_lookup(&self, m: &Rc<Material>) -> Option<usize> {
        self.mats.iter().position(|q| Rc::ptr_eq(q, m))
    }

    fn mat_id(&mut self, m: &Rc<Material>) -> usize {
        if let Some(i) = self.mat_lookup(m) {
            return i;
        }
        self.mats.push(m.clone());
        self.mats.len() - 1
    }

    fn new_shape(&mut self, m: &Rc<Material>, o: &O) -> i16 {
        let id = self.shapes.len() as i16;
        let mat = self.mat_id(m);
        self.shapes.push(ShapeInfo {
            mat,
            line: o.line.unwrap_or(true),
            line_same: o.line_same,
            cast: o.cast,
            joins: o.joins.clone(),
            rim_over: o.rim_over,
        });
        id
    }

    fn write(&mut self, x: i32, y: i32, id: i16, nx: f64, ny: f64, nz: f64, o: &O) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        if let Some(masks) = &o.mask {
            if self.mat[i] < 0
                || !masks
                    .iter()
                    .any(|m| self.mat_lookup(m).map(|k| k as i16) == Some(self.mat[i]))
            {
                return;
            }
        }
        if o.under && self.mat[i] >= 0 {
            return;
        }
        if let Some(f) = o.where_ {
            if !f(x, y) {
                return;
            }
        }
        self.mat[i] = self.shapes[id as usize].mat as i16;
        self.shape[i] = id;
        let mut v = [nx, ny, nz];
        if let Some(r) = o.round {
            v = norm3([nx * r, ny * r, nz]);
        }
        if let Some(t) = o.tilt {
            v = norm3([v[0] + t[0], v[1] + t[1], v[2]]);
        }
        self.nx[i] = v[0] as f32;
        self.ny[i] = v[1] as f32;
        self.nz[i] = v[2] as f32;
        self.tone[i] = o.tone as f32;
    }

    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, m: &Rc<Material>, o: O) -> i16 {
        let id = self.new_shape(m, &o);
        for y in (cy - ry - 1.0).floor() as i32..=(cy + ry + 1.0).floor() as i32 {
            for x in (cx - rx - 1.0).floor() as i32..=(cx + rx + 1.0).floor() as i32 {
                let dx = (x as f64 + 0.5 - cx) / rx;
                let dy = (y as f64 + 0.5 - cy) / ry;
                let d2 = dx * dx + dy * dy;
                if d2 > 1.0 {
                    continue;
                }
                if o.flat {
                    let n = o.normal.unwrap_or([0.0, 0.0, 1.0]);
                    self.write(x, y, id, n[0], n[1], n[2], &o);
                } else {
                    self.write(x, y, id, dx, dy, (1.0 - d2).max(0.0).sqrt(), &o);
                }
            }
        }
        id
    }

    /// Tapered capsule with a cylindrical normal (limbs, horns, strands).
    #[allow(clippy::too_many_arguments)]
    pub fn capsule(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        r0: f64,
        r1: f64,
        m: &Rc<Material>,
        o: O,
    ) -> i16 {
        let id = self.new_shape(m, &o);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = {
            let l = dx * dx + dy * dy;
            if l == 0.0 {
                1e-6
            } else {
                l
            }
        };
        let rr = r0.max(r1);
        for y in (y0.min(y1) - rr - 1.0).floor() as i32..=(y0.max(y1) + rr + 1.0).floor() as i32 {
            for x in (x0.min(x1) - rr - 1.0).floor() as i32..=(x0.max(x1) + rr + 1.0).floor() as i32
            {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let t = clamp(((px - x0) * dx + (py - y0) * dy) / len2, 0.0, 1.0);
                let (cx, cy) = (x0 + dx * t, y0 + dy * t);
                let r = r0 + (r1 - r0) * t;
                let (ex, ey) = (px - cx, py - cy);
                let d2 = (ex * ex + ey * ey) / (r * r);
                if d2 > 1.0 {
                    continue;
                }
                if o.flat {
                    let n = o.normal.unwrap_or([0.0, 0.0, 1.0]);
                    self.write(x, y, id, n[0], n[1], 1.0, &o);
                } else {
                    self.write(x, y, id, ex / r, ey / r, (1.0 - d2).max(0.0).sqrt(), &o);
                }
            }
        }
        id
    }

    /// Smooth tapered stroke through points (hair locks, cloth folds).
    pub fn stroke(&mut self, points: &[P2], w0: f64, w1: f64, m: &Rc<Material>, o: O) -> i16 {
        let pts = catmull(points, o.steps.unwrap_or(6));
        let id = self.new_shape(m, &o);
        let total = pts.len() - 1;
        let mut segs: Vec<[f64; 6]> = Vec::new();
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for k in 0..total {
            let [x0, y0] = pts[k];
            let [x1, y1] = pts[k + 1];
            let r0 = w0 + (w1 - w0) * (k as f64 / total as f64);
            let r1 = w0 + (w1 - w0) * ((k + 1) as f64 / total as f64);
            segs.push([x0, y0, x1, y1, r0, r1]);
            let rr = r0.max(r1);
            min_x = min_x.min(x0 - rr).min(x1 - rr);
            max_x = max_x.max(x0 + rr).max(x1 + rr);
            min_y = min_y.min(y0 - rr).min(y1 - rr);
            max_y = max_y.max(y0 + rr).max(y1 + rr);
        }
        let bx = min_x.floor() as i32 - 1;
        let by = min_y.floor() as i32 - 1;
        let bw = max_x.ceil() as i32 - bx + 2;
        let bh = max_y.ceil() as i32 - by + 2;
        let cells = (bw * bh) as usize;
        let mut best = vec![f32::INFINITY; cells];
        let mut nxs = vec![0.0f32; cells];
        let mut nys = vec![0.0f32; cells];
        for [x0, y0, x1, y1, r0, r1] in segs {
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len2 = {
                let l = dx * dx + dy * dy;
                if l == 0.0 {
                    1e-6
                } else {
                    l
                }
            };
            let rr = r0.max(r1);
            let ya = by.max((y0.min(y1) - rr - 1.0).floor() as i32);
            let yb = (by + bh - 1).min((y0.max(y1) + rr + 1.0).ceil() as i32);
            let xa = bx.max((x0.min(x1) - rr - 1.0).floor() as i32);
            let xb = (bx + bw - 1).min((x0.max(x1) + rr + 1.0).ceil() as i32);
            for y in ya..=yb {
                for x in xa..=xb {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let t = clamp(((px - x0) * dx + (py - y0) * dy) / len2, 0.0, 1.0);
                    let r = r0 + (r1 - r0) * t;
                    let (ex, ey) = (px - (x0 + dx * t), py - (y0 + dy * t));
                    let d2 = (ex * ex + ey * ey) / (r * r);
                    let j = ((y - by) * bw + (x - bx)) as usize;
                    if d2 < best[j] as f64 {
                        best[j] = d2 as f32;
                        nxs[j] = (ex / r) as f32;
                        nys[j] = (ey / r) as f32;
                    }
                }
            }
        }
        let flat = 1.0 - o.flatten * 0.5;
        for j in 0..cells {
            if best[j] as f64 > 1.0 {
                continue;
            }
            let nx = nxs[j] as f64 * flat;
            let ny = nys[j] as f64 * flat;
            let x = bx + (j as i32 % bw);
            let y = by + (j as i32 / bw);
            self.write(
                x,
                y,
                id,
                nx,
                ny,
                (1.0 - nx * nx - ny * ny).max(0.05).sqrt(),
                &o,
            );
        }
        id
    }

    /// Filled polygon. `o.shade`: Flat (o.normal), Cyl (rounded across each
    /// row), Dome (rounded both ways) or VCyl (rounded down each column).
    pub fn poly(&mut self, points: &[P2], m: &Rc<Material>, o: O) -> i16 {
        let id = self.new_shape(m, &o);
        let mut y0 = f64::INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for p in points {
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        let y0 = 0.max(y0.floor() as i32);
        let y1 = (self.h as i32 - 1).min(y1.ceil() as i32);
        let mode = o.shade.unwrap_or(Shade::Cyl);
        let mut spans: Vec<(i32, i32, i32)> = Vec::new();
        for y in y0..=y1 {
            let py = y as f64 + 0.5;
            let mut xs: Vec<f64> = Vec::new();
            let mut j = points.len() - 1;
            for i in 0..points.len() {
                let [ax, ay] = points[i];
                let [bx, by] = points[j];
                if (ay > py) != (by > py) {
                    xs.push(((bx - ax) * (py - ay)) / (by - ay) + ax);
                }
                j = i;
            }
            xs.sort_by(|a, b| a.total_cmp(b));
            let mut k = 0;
            while k + 1 < xs.len() {
                let a = (xs[k] - 0.5).ceil() as i32;
                let b = (xs[k + 1] - 0.5).floor() as i32;
                if b >= a {
                    spans.push((y, a, b));
                }
                k += 2;
            }
        }
        if spans.is_empty() {
            return id;
        }
        let mut row_min: HashMap<i32, i32> = HashMap::new();
        let mut row_max: HashMap<i32, i32> = HashMap::new();
        let mut col_min: HashMap<i32, i32> = HashMap::new();
        let mut col_max: HashMap<i32, i32> = HashMap::new();
        for &(y, a, b) in &spans {
            let e = row_min.entry(y).or_insert(a);
            *e = (*e).min(a);
            let e = row_max.entry(y).or_insert(b);
            *e = (*e).max(b);
            if mode == Shade::VCyl || mode == Shade::Dome {
                for x in a..=b {
                    let e = col_min.entry(x).or_insert(y);
                    *e = (*e).min(y);
                    let e = col_max.entry(x).or_insert(y);
                    *e = (*e).max(y);
                }
            }
        }
        let k = o.curve.unwrap_or(0.85);
        for &(y, a0, b0) in &spans {
            let a = row_min[&y];
            let b = row_max[&y];
            for x in a0..=b0 {
                if mode == Shade::Flat {
                    let n = o.normal.unwrap_or([0.0, 0.0, 1.0]);
                    self.write(x, y, id, n[0], n[1], n[2], &o);
                    continue;
                }
                let (mut nx, mut ny) = (0.0, 0.0);
                if mode == Shade::Cyl || mode == Shade::Dome {
                    nx = if b > a {
                        ((x - a) as f64 / (b - a) as f64) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                }
                if mode == Shade::VCyl || mode == Shade::Dome {
                    let c0 = col_min[&x];
                    let c1 = col_max[&x];
                    ny = if c1 > c0 {
                        ((y - c0) as f64 / (c1 - c0) as f64) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                }
                nx *= k;
                ny *= k;
                self.write(
                    x,
                    y,
                    id,
                    nx,
                    ny,
                    (1.0 - nx * nx - ny * ny).max(0.05).sqrt(),
                    &o,
                );
            }
        }
        id
    }

    /// A signed-distance shape evaluated over `box_` [x0, y0, x1, y1]. The
    /// field returns (distance, thickness); normals come from its gradient,
    /// inflated by the thickness, so fused limbs shade as one surface.
    pub fn sdf(
        &mut self,
        m: &Rc<Material>,
        field: &dyn Fn(f64, f64) -> (f64, f64),
        box_: [f64; 4],
        o: O,
    ) -> i16 {
        let id = self.new_shape(m, &o);
        let x0 = 0.max(box_[0].floor() as i32 - 1);
        let y0 = 0.max(box_[1].floor() as i32 - 1);
        let x1 = (self.w as i32 - 1).min(box_[2].ceil() as i32 + 1);
        let y1 = (self.h as i32 - 1).min(box_[3].ceil() as i32 + 1);
        let bw = x1 - x0 + 3;
        let bh = y1 - y0 + 3;
        let mut d_buf = vec![0.0f32; (bw * bh) as usize];
        let mut r_buf = vec![0.0f32; (bw * bh) as usize];
        for j in 0..bh {
            for i in 0..bw {
                let (d, r) = field((x0 - 1 + i) as f64 + 0.5, (y0 - 1 + j) as f64 + 0.5);
                d_buf[(j * bw + i) as usize] = d as f32;
                r_buf[(j * bw + i) as usize] = r as f32;
            }
        }
        let curve = o.curve.unwrap_or(1.0);
        let soft = o.soft.unwrap_or(1.0);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let k = ((y - y0 + 1) * bw + (x - x0 + 1)) as usize;
                let d = d_buf[k] as f64;
                if d > 0.0 {
                    continue;
                }
                let mut gx = d_buf[k + 1] as f64 - d_buf[k - 1] as f64;
                let mut gy = d_buf[k + bw as usize] as f64 - d_buf[k - bw as usize] as f64;
                let gl = gx.hypot(gy);
                let gl = if gl == 0.0 { 1.0 } else { gl };
                gx /= gl;
                gy /= gl;
                let e = clamp(1.0 + d / 0.5f64.max(r_buf[k] as f64), 0.0, 1.0).powf(soft) * curve;
                self.write(x, y, id, gx * e, gy * e, (1.0 - e * e).max(0.02).sqrt(), &o);
            }
        }
        id
    }

    /// Paint a garment region onto shapes already drawn: pixels where
    /// `where_` holds change material but keep their form and shape.
    pub fn recolor(&mut self, m: &Rc<Material>, where_: &dyn Fn(i32, i32) -> bool, o: Recolor) {
        let id = self.mat_id(m) as i16;
        let only: Option<Vec<i16>> = o
            .only
            .as_ref()
            .map(|list| list.iter().map(|q| self.mat_id(q) as i16).collect());
        let [x0, y0, x1, y1] =
            o.box_
                .unwrap_or([0.0, 0.0, self.w as f64 - 1.0, self.h as f64 - 1.0]);
        for y in 0.max(y0.floor() as i32)..=(self.h as i32 - 1).min(y1.floor() as i32) {
            for x in 0.max(x0.floor() as i32)..=(self.w as i32 - 1).min(x1.floor() as i32) {
                let i = y as usize * self.w + x as usize;
                if self.mat[i] < 0 {
                    continue;
                }
                if let Some(only) = &only {
                    if !only.contains(&self.mat[i]) {
                        continue;
                    }
                }
                if let Some(shapes) = &o.shapes {
                    if !shapes.contains(&self.shape[i]) {
                        continue;
                    }
                }
                if !where_(x, y) {
                    continue;
                }
                self.mat[i] = id;
                if o.tone != 0.0 {
                    self.tone[i] = (self.tone[i] as f64 + o.tone) as f32;
                }
            }
        }
    }

    /// Add `f(x, y)` to the light of every pixel of one shape.
    pub fn shade_where(&mut self, id: i16, f: &dyn Fn(f64, f64) -> f64) {
        for i in 0..self.shape.len() {
            if self.shape[i] != id {
                continue;
            }
            self.tone[i] =
                (self.tone[i] as f64 + f((i % self.w) as f64, (i / self.w) as f64)) as f32;
        }
    }

    /// Darken what lies just below a shape's lower edge (jaw, bust, fringe).
    pub fn shade_below(&mut self, id: i16, len: i32, depth: f64) {
        for x in 0..self.w {
            let mut bottom: i32 = -1;
            for y in (0..self.h).rev() {
                if self.shape[y * self.w + x] == id {
                    bottom = y as i32;
                    break;
                }
            }
            if bottom < 0 {
                continue;
            }
            for k in 1..=len {
                let y = bottom + k;
                if y >= self.h as i32 {
                    break;
                }
                let i = y as usize * self.w + x;
                if self.mat[i] < 0 || self.shape[i] == id {
                    continue;
                }
                self.tone[i] =
                    (self.tone[i] as f64 - depth * (1.0 - (k - 1) as f64 / len as f64)) as f32;
            }
        }
    }

    /// Shape id and material under a pixel (for garment logic).
    pub fn at(&self, x: i32, y: i32) -> Option<(i16, Rc<Material>)> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return None;
        }
        let i = y as usize * self.w + x as usize;
        (self.mat[i] >= 0).then(|| (self.shape[i], self.mats[self.mat[i] as usize].clone()))
    }

    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, m: &Rc<Material>, o: O) -> i16 {
        let o = if o.shade.is_none() {
            o.shade(Shade::Flat)
        } else {
            o
        };
        self.poly(&[[x, y], [x + w, y], [x + w, y + h], [x, y + h]], m, o)
    }

    /// Fabric pattern: `f(x, y)` returns a light offset for every pixel of
    /// material `m` painted so far (later shapes are not patterned).
    pub fn texture(&mut self, m: &Rc<Material>, f: Box<dyn Fn(i32, i32) -> f64>) {
        let mat = self.mat_id(m);
        self.textures.push(Texture {
            mat,
            z: self.shapes.len() as i32,
            f,
        });
    }

    /// A crease through the points: dark valley, lit ridge toward the light.
    pub fn fold(&mut self, points: &[P2], depth: f64, ridge: f64) {
        let pts = catmull(points, 5);
        let sign = if self.light[0] > 0.0 {
            1.0
        } else if self.light[0] < 0.0 {
            -1.0
        } else {
            0.0
        };
        let lx = {
            let v = -sign;
            if v == 0.0 {
                -1
            } else {
                v as i32
            }
        };
        let mut seen: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for k in 0..pts.len().saturating_sub(1) {
            let [x0, y0] = pts[k];
            let [x1, y1] = pts[k + 1];
            let n = 1.max(((x1 - x0).hypot(y1 - y0) * 1.5).ceil() as i32);
            for j in 0..=n {
                let x = (x0 + ((x1 - x0) * j as f64) / n as f64).floor() as i32;
                let y = (y0 + ((y1 - y0) * j as f64) / n as f64).floor() as i32;
                let key = y as i64 * self.w as i64 + x as i64;
                if seen.contains(&key)
                    || x < 0
                    || y < 0
                    || x >= self.w as i32
                    || y >= self.h as i32
                    || self.mat[key as usize] < 0
                {
                    continue;
                }
                seen.insert(key);
                self.tone[key as usize] = (self.tone[key as usize] as f64 - depth) as f32;
                let r = key - lx as i64;
                if x - lx >= 0
                    && x - lx < self.w as i32
                    && r >= 0
                    && (r as usize) < self.mat.len()
                    && self.mat[r as usize] == self.mat[key as usize]
                {
                    self.tone[r as usize] = (self.tone[r as usize] as f64 + depth * ridge) as f32;
                }
            }
        }
    }

    /// Hand-placed pixels drawn after shading: rows of chars keyed by `pal`.
    pub fn stamp(&mut self, x: f64, y: f64, rows: &[&str], pal: &[(char, Rgb)], flip: bool) {
        let z = self.shapes.len() as i32;
        for (j, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            for (i, &ch) in chars.iter().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let Some(&(_, color)) = pal.iter().find(|(c, _)| *c == ch) else {
                    continue;
                };
                let px = if flip {
                    x + chars.len() as f64 - 1.0 - i as f64
                } else {
                    x + i as f64
                };
                self.details
                    .push((trunc(px), trunc(y + j as f64), color, z));
            }
        }
    }

    pub fn px(&mut self, x: f64, y: f64, c: Rgb) {
        self.details
            .push((trunc(x), trunc(y), c, self.shapes.len() as i32));
    }

    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb) {
        let (mut x0, mut y0, x1, y1) = (trunc(x0), trunc(y0), trunc(x1), trunc(y1));
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let z = self.shapes.len() as i32;
        loop {
            self.details.push((x0, y0, c, z));
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Light, shade, outline and detail the figure into a transparent surface.
    pub fn render(&self) -> Surface {
        let (w, h) = (self.w as i32, self.h as i32);
        let mut out = Surface::new(self.w, self.h);
        let mut tone = self.tone.clone();
        let light = self.light;
        // Cast shadows: a front shape above-left darkens what lies behind it.
        let (lx, ly) = (light[0], light[1]);
        let ll = {
            let l = lx.hypot(ly);
            if l == 0.0 {
                1.0
            } else {
                l
            }
        };
        let (sx, sy) = (lx / ll, ly / ll);
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                let s = self.shape[i];
                if s < 0 {
                    continue;
                }
                for k in 1..=4 {
                    let qx = jr(x as f64 + sx * k as f64) as i32;
                    let qy = jr(y as f64 + sy * k as f64) as i32;
                    if qx < 0 || qy < 0 || qx >= w || qy >= h {
                        break;
                    }
                    let t = self.shape[(qx + qy * w) as usize];
                    if t > s
                        && self.shapes[t as usize].cast >= k
                        && self.shapes[t as usize].mat != self.shapes[s as usize].mat
                    {
                        tone[i] = (tone[i] as f64 - 0.28) as f32;
                        break;
                    }
                    if t > s
                        && self.shapes[t as usize].cast >= k
                        && self.shapes[t as usize].mat == self.shapes[s as usize].mat
                    {
                        tone[i] = (tone[i] as f64 - 0.16) as f32;
                        break;
                    }
                }
            }
        }
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                let m = self.mat[i];
                if m < 0 {
                    continue;
                }
                let mm = &self.mats[m as usize];
                let mut v = self.nx[i] as f64 * light[0]
                    + self.ny[i] as f64 * light[1]
                    + self.nz[i] as f64 * light[2];
                v = (v * 0.5 + 0.5) * mm.contrast + mm.bias + tone[i] as f64;
                for t in &self.textures {
                    if t.mat as i16 == m && (self.shape[i] as i32) < t.z {
                        v += (t.f)(x, y);
                    }
                }
                let mut c = pick(&mm.ramp, clamp(v, 0.0, 1.0), x, y, mm.dither);
                if let Some(rim) = &self.rim {
                    if mm.rim {
                        let (qx, qy) = (x + rim.dir[0], y + rim.dir[1]);
                        let outside = qx < 0
                            || qy < 0
                            || qx >= w
                            || qy >= h
                            || self.mat[(qy * w + qx) as usize] < 0;
                        // `rim_over` shapes also catch the rim light where they
                        // pass in front of something darker behind them.
                        let over = !outside
                            && self.shapes[self.shape[i] as usize].rim_over
                            && self.shape[(qy * w + qx) as usize] < self.shape[i]
                            && self.mat[(qy * w + qx) as usize] != m;
                        if outside || over {
                            c = mix(c, rim.color, if over { rim.k * 0.8 } else { rim.k });
                        }
                    }
                }
                out.set_i(x, y, c, 1.0);
            }
        }
        // Inner contours: the border of a front shape over a different one.
        let mut inner: Vec<(i32, i32, Rgb)> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                let s = self.shape[i];
                if s < 0 || !self.shapes[s as usize].line {
                    continue;
                }
                if let Some(joins) = &self.shapes[s as usize].joins {
                    if joins.iter().any(|j| {
                        (x as f64 + 0.5 - j[0]).powi(2) + (y as f64 + 0.5 - j[1]).powi(2)
                            < j[2] * j[2]
                    }) {
                        continue;
                    }
                }
                for n in 0..4 {
                    let (qx, qy) = (x + DX[n], y + DY[n]);
                    if qx < 0 || qy < 0 || qx >= w || qy >= h {
                        continue;
                    }
                    let t = self.shape[(qy * w + qx) as usize];
                    if t < 0 || t >= s {
                        continue;
                    }
                    if self.mat[(qy * w + qx) as usize] == self.mat[i]
                        && !self.shapes[s as usize].line_same
                    {
                        continue;
                    }
                    inner.push((x, y, self.mats[self.mat[i] as usize].inner));
                    break;
                }
            }
        }
        for (x, y, c) in inner {
            out.set_i(x, y, c, 1.0);
        }
        // Outer outline, tinted by the material it wraps (selective outline).
        let mut outer: Vec<(i32, i32, Rgb)> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if self.mat[(y * w + x) as usize] >= 0 {
                    continue;
                }
                let mut best: i32 = -1;
                let mut lit = false;
                for n in 0..4 {
                    let (qx, qy) = (x + DX[n], y + DY[n]);
                    if qx < 0 || qy < 0 || qx >= w || qy >= h {
                        continue;
                    }
                    let t = self.shape[(qy * w + qx) as usize] as i32;
                    if t > best {
                        best = t;
                        lit = -(DX[n] as f64) * sx - (DY[n] as f64) * sy > 0.35;
                    }
                }
                if best >= 0 {
                    let mm = &self.mats[self.shapes[best as usize].mat];
                    if !mm.outline {
                        continue;
                    }
                    let line = mm.line;
                    let second = if mm.ramp.len() > 1 {
                        mm.ramp[1]
                    } else {
                        mm.ramp[0]
                    };
                    outer.push((x, y, if lit { mix(line, second, 0.5) } else { line }));
                }
            }
        }
        for (x, y, c) in outer {
            out.set_i(x, y, c, 1.0);
        }
        // Details sit at the depth they were placed: later shapes cover them.
        for &(x, y, c, z) in &self.details {
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            if (self.shape[(y * w + x) as usize] as i32) < z {
                out.set_i(x, y, c, 1.0);
            }
        }
        out
    }
}

/// Catmull-Rom through the points, `steps` samples per span.
pub fn catmull(points: &[P2], steps: usize) -> Vec<P2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = Vec::new();
    let mut p: Vec<P2> = Vec::with_capacity(points.len() + 2);
    p.push(points[0]);
    p.extend_from_slice(points);
    p.push(points[points.len() - 1]);
    for i in 1..p.len() - 2 {
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let (t2, t3) = (t * t, t * t * t);
            let f = |k: usize| {
                0.5 * (2.0 * p[i][k]
                    + (-p[i - 1][k] + p[i + 1][k]) * t
                    + (2.0 * p[i - 1][k] - 5.0 * p[i][k] + 4.0 * p[i + 1][k] - p[i + 2][k]) * t2
                    + (-p[i - 1][k] + 3.0 * p[i][k] - 3.0 * p[i + 1][k] + p[i + 2][k]) * t3)
            };
            out.push([f(0), f(1)]);
        }
    }
    out.push(points[points.len() - 1]);
    out
}

/// Memoised sprite renders by pose: poses repeat, so frames are mostly hits.
pub struct PoseCache {
    map: HashMap<String, Rc<Surface>>,
    order: Vec<String>,
    size: usize,
}

impl PoseCache {
    pub fn new(size: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::new(),
            size,
        }
    }

    pub fn get(&mut self, key: String, draw: impl FnOnce() -> Surface) -> Rc<Surface> {
        if let Some(hit) = self.map.get(&key) {
            let hit = hit.clone();
            self.order.retain(|k| k != &key);
            self.order.push(key);
            return hit;
        }
        let hit = Rc::new(draw());
        self.map.insert(key.clone(), hit.clone());
        self.order.push(key);
        if self.order.len() > self.size {
            let old = self.order.remove(0);
            self.map.remove(&old);
        }
        hit
    }
}
