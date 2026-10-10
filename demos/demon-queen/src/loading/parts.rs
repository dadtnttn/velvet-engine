//! 2D vector helpers, two-bone arm IK, and hands/cuffs built as fused distance
//! fields (ports of `parts.js` and `anatomy.js`).
use std::rc::Rc;

use super::pixel::{clamp, jr, Figure, Material, Recolor, Rgb, O, P2};
use super::sdf::{capsule, poly, smin, Field};

pub fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
pub fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn mul(a: P2, k: f64) -> P2 {
    [a[0] * k, a[1] * k]
}
pub fn unit(a: P2) -> P2 {
    let l = a[0].hypot(a[1]);
    let l = if l == 0.0 { 1.0 } else { l };
    [a[0] / l, a[1] / l]
}
pub fn perp(a: P2) -> P2 {
    [-a[1], a[0]]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// Two-bone IK: elbow for a limb from `s` to `h`, bending toward `bend`.
pub fn elbow(s: P2, h: P2, l1: f64, l2: f64, bend: P2) -> P2 {
    let (dx, dy) = (h[0] - s[0], h[1] - s[1]);
    let d = clamp(dx.hypot(dy), 1.0, l1 + l2 - 0.5);
    let a = dy.atan2(dx);
    let b = clamp((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d), -1.0, 1.0).acos();
    let e1 = [s[0] + (a + b).cos() * l1, s[1] + (a + b).sin() * l1];
    let e2 = [s[0] + (a - b).cos() * l1, s[1] + (a - b).sin() * l1];
    let score = |e: P2| (e[0] - s[0]) * bend[0] + (e[1] - s[1]) * bend[1];
    if score(e1) > score(e2) {
        e1
    } else {
        e2
    }
}

fn box_of(points: &[P2], pad: f64) -> [f64; 4] {
    let xs = points.iter().map(|q| q[0]);
    let ys = points.iter().map(|q| q[1]);
    [
        xs.clone().fold(f64::INFINITY, f64::min) - pad,
        ys.clone().fold(f64::INFINITY, f64::min) - pad,
        xs.fold(f64::NEG_INFINITY, f64::max) + pad,
        ys.fold(f64::NEG_INFINITY, f64::max) + pad,
    ]
}

fn cap(a: P2, b: P2, ra: f64, rb: f64) -> Field {
    capsule(a[0], a[1], b[0], b[1], ra, rb, None)
}

#[derive(Default)]
pub struct FistOpts {
    pub size: Option<f64>,
    pub nail: Option<Rgb>,
    pub ring: Option<[Rgb; 2]>,
}

/// A fist closed around something running along `axis` through `g`. `wrist`
/// tells which side the back of the hand is on.
pub fn fist(fig: &mut Figure, g: P2, axis: P2, wrist: P2, skin: &Rc<Material>, o: FistOpts) -> i16 {
    let a = unit(axis);
    let mut p = perp(a);
    if dot(p, sub(wrist, g)) > 0.0 {
        p = mul(p, -1.0);
    }
    let s = o.size.unwrap_or(1.0);
    let palm = add(g, mul(p, -3.0 * s));
    let mut fields: Vec<Field> = vec![
        cap(
            add(palm, mul(a, -3.0 * s)),
            add(palm, mul(a, 3.0 * s)),
            4.2 * s,
            4.2 * s,
        ),
        cap(wrist, palm, 3.3 * s, 4.0 * s),
    ];
    let mut tips: Vec<P2> = Vec::new();
    for i in 0..4 {
        let q = add(g, mul(a, (i as f64 - 1.5) * 2.5 * s));
        let from = add(q, mul(p, -2.0 * s));
        let to = add(q, mul(p, (3.0 - (i as f64 - 1.5).abs() * 0.55) * s));
        fields.push(cap(from, to, 1.75 * s, 1.5 * s));
        tips.push(to);
    }
    let t0 = add(add(g, mul(p, -3.6 * s)), mul(a, -4.4 * s));
    let t1 = add(add(g, mul(p, 0.4 * s)), mul(a, -5.8 * s));
    fields.push(cap(t0, t1, 2.0 * s, 1.6 * s));
    let mut pts = vec![wrist, g, t0, t1];
    pts.extend(tips.iter().copied());
    let field = smin(1.2, fields);
    let id = fig.sdf(
        skin,
        &*field,
        box_of(&pts, 7.0),
        O::new().cast(1).curve(0.9),
    );
    let gap = skin.inner;
    for i in 1..4 {
        let q = add(g, mul(a, (i as f64 - 2.0) * 2.5 * s));
        for k in [0.4, 1.4, 2.4] {
            let c = add(q, mul(p, k * s));
            fig.px(jr(c[0]), jr(c[1]), gap);
        }
    }
    // The thumb's edge over the fingers.
    for k in 0..=3 {
        let c = add(
            add(g, mul(p, (-2.6 + k as f64 * 0.9) * s)),
            mul(a, -3.4 * s),
        );
        fig.px(jr(c[0]), jr(c[1]), gap);
    }
    if let Some(nail) = o.nail {
        for t in &tips {
            let c = add(*t, mul(p, 1.3 * s));
            fig.px(jr(c[0]), jr(c[1]), nail);
        }
    }
    if let Some(ring) = o.ring {
        let c = add(add(g, mul(a, -1.25 * s)), mul(p, -1.2 * s));
        fig.px(jr(c[0]), jr(c[1]), ring[0]);
        fig.px(jr(c[0] + a[0]), jr(c[1] + a[1]), ring[1]);
    }
    id
}

#[derive(Default)]
pub struct CuffOpts {
    pub len: Option<f64>,
    pub w0: Option<f64>,
    pub w1: Option<f64>,
    pub w: Option<f64>,
    pub buttons: Option<Rgb>,
}

/// A sleeve end flaring past the wrist, its lining showing at the rim.
pub fn bell_cuff(
    fig: &mut Figure,
    wrist: P2,
    d: P2,
    cloth: &Rc<Material>,
    lining: Option<&Rc<Material>>,
    trim: Option<[Rgb; 2]>,
    o: CuffOpts,
) -> i16 {
    let n = perp(d);
    let back = add(wrist, mul(d, -o.len.unwrap_or(6.5)));
    let front = add(wrist, mul(d, 1.2));
    let (w0, w1) = (o.w0.unwrap_or(4.2), o.w1.unwrap_or(6.4));
    let pts = vec![
        add(back, mul(n, w0)),
        add(front, mul(n, w1)),
        add(front, mul(n, -w1)),
        add(back, mul(n, -w0)),
    ];
    let field = poly(pts.clone(), 3.2);
    let id = fig.sdf(
        cloth,
        &*field,
        box_of(&pts, 2.0),
        O::new().line_same().cast(1).curve(0.9),
    );
    if let Some(lining) = lining {
        fig.recolor(
            lining,
            &|x, y| dot(sub([x as f64 + 0.5, y as f64 + 0.5], front), d) > -1.8,
            Recolor::shapes(&[id]),
        );
    }
    if let Some(trim) = trim {
        let mut k = -w1 + 0.5;
        while k <= w1 - 0.5 {
            let c = add(add(front, mul(d, -2.2)), mul(n, k));
            let which = if (jr(k * 2.0) as i32) & 2 != 0 { 0 } else { 1 };
            fig.px(jr(c[0]), jr(c[1]), trim[which]);
            k += 0.5;
        }
    }
    id
}

/// A plain fitted cuff band (buttons optional) around the wrist.
pub fn band_cuff(fig: &mut Figure, wrist: P2, d: P2, cloth: &Rc<Material>, o: CuffOpts) -> i16 {
    let n = perp(d);
    let back = add(wrist, mul(d, -o.len.unwrap_or(5.0)));
    let front = add(wrist, mul(d, 0.8));
    let w = o.w.unwrap_or(4.6);
    let pts = vec![
        add(back, mul(n, w)),
        add(front, mul(n, w + 0.4)),
        add(front, mul(n, -w - 0.4)),
        add(back, mul(n, -w)),
    ];
    let field = poly(pts.clone(), 3.0);
    let id = fig.sdf(
        cloth,
        &*field,
        box_of(&pts, 2.0),
        O::new().cast(1).curve(0.9),
    );
    if let Some(b) = o.buttons {
        for k in [-1.6, 1.6] {
            let c = add(add(back, mul(d, 2.4)), mul(n, k));
            fig.px(jr(c[0]), jr(c[1]), b);
        }
    }
    id
}
