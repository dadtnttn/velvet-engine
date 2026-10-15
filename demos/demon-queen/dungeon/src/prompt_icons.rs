//! Interaction prompts shared by the world, its overlay and the caverns: what
//! the F key does here, told by an icon and a colour of its own, a verb and
//! the name of what it acts on. Icons are flat vector shapes in a unit box
//! (x right, y down, both in -0.5..0.5) so the overlay can fill them and the
//! world can extrude them into floating medallions.
use std::f32::consts::{PI, TAU};

/// What an interaction is about: picks its icon and accent colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Door,
    Leave,
    Talk,
    Shop,
    Wardrobe,
    Mine,
    Castle,
    House,
    Forest,
    Chest,
    Loot,
    Rune,
    Rest,
    Memory,
    Crack,
    Lock,
    Tavern,
    Bath,
    Council,
    Heart,
    // Menu icons.
    Play,
    Disk,
    Folder,
    Backpack,
    Gear,
    Power,
    Crown,
    Feast,
}

impl Icon {
    pub const ALL: [Icon; 28] = [
        Icon::Door,
        Icon::Leave,
        Icon::Talk,
        Icon::Shop,
        Icon::Wardrobe,
        Icon::Mine,
        Icon::Castle,
        Icon::House,
        Icon::Forest,
        Icon::Chest,
        Icon::Loot,
        Icon::Rune,
        Icon::Rest,
        Icon::Memory,
        Icon::Crack,
        Icon::Lock,
        Icon::Tavern,
        Icon::Bath,
        Icon::Council,
        Icon::Heart,
        Icon::Play,
        Icon::Disk,
        Icon::Folder,
        Icon::Backpack,
        Icon::Gear,
        Icon::Power,
        Icon::Crown,
        Icon::Feast,
    ];

    /// Position in [`Icon::ALL`] (for per-icon mesh tables).
    pub fn index(self) -> usize {
        Icon::ALL.iter().position(|&i| i == self).unwrap_or(0)
    }

    /// The icon's own colour (linear RGB, bright enough to glow).
    pub fn accent(self) -> [f32; 3] {
        match self {
            Icon::Door => [0.95, 0.58, 0.26],
            Icon::Leave => [0.32, 0.84, 0.64],
            Icon::Talk => [0.70, 0.50, 1.0],
            Icon::Shop => [1.0, 0.82, 0.30],
            Icon::Wardrobe => [1.0, 0.48, 0.66],
            Icon::Mine => [0.98, 0.50, 0.20],
            Icon::Castle => [0.48, 0.68, 1.0],
            Icon::House => [0.58, 0.86, 0.42],
            Icon::Forest => [0.40, 0.84, 0.48],
            Icon::Chest => [1.0, 0.74, 0.28],
            Icon::Loot => [1.0, 0.88, 0.42],
            Icon::Rune => [0.38, 0.90, 1.0],
            Icon::Rest => [1.0, 0.62, 0.26],
            Icon::Memory => [0.80, 0.64, 1.0],
            Icon::Crack => [0.82, 0.76, 0.64],
            Icon::Lock => [0.96, 0.38, 0.34],
            Icon::Tavern => [0.98, 0.70, 0.30],
            Icon::Bath => [0.46, 0.82, 0.98],
            Icon::Council => [0.92, 0.80, 0.52],
            Icon::Heart => [1.0, 0.42, 0.56],
            Icon::Play => [0.52, 0.92, 0.62],
            Icon::Disk => [0.55, 0.74, 1.0],
            Icon::Folder => [1.0, 0.78, 0.44],
            Icon::Backpack => [0.92, 0.62, 0.38],
            Icon::Gear => [0.74, 0.78, 0.86],
            Icon::Power => [0.98, 0.5, 0.5],
            Icon::Crown => [1.0, 0.9, 0.5],
            Icon::Feast => [0.9, 0.55, 0.7],
        }
    }
}

/// One F prompt: the icon, the action and, when useful, what it acts on.
#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    pub icon: Icon,
    pub verb: String,
    pub detail: Option<String>,
    /// False when F does nothing here yet (a sealed threshold): no key shown.
    pub keyed: bool,
    /// A second thing to do here, on its own key: (key, verb).
    pub second: Option<(String, String)>,
}

impl Prompt {
    pub fn new(icon: Icon, verb: impl Into<String>) -> Self {
        Self {
            icon,
            verb: verb.into(),
            detail: None,
            keyed: true,
            second: None,
        }
    }

    /// A second action beside the first, on its own key.
    pub fn also(mut self, key: impl Into<String>, verb: impl Into<String>) -> Self {
        self.second = Some((key.into(), verb.into()));
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// A notice rather than an action: shown without the F key.
    pub fn locked(mut self) -> Self {
        self.keyed = false;
        self
    }

    /// Plain text form, for logs, tests and the window title.
    pub fn text(&self) -> String {
        let key = if self.keyed { "F  " } else { "" };
        let main = match &self.detail {
            Some(detail) => format!("{key}{} · {detail}", self.verb),
            None => format!("{key}{}", self.verb),
        };
        match &self.second {
            Some((k, verb)) => format!("{main}   {k}  {verb}"),
            None => main,
        }
    }
}

/// One filled region of an icon. `ink` 1 is the icon colour; 0 cuts back to
/// the medallion's face (holes, eyes, keyholes), drawn over earlier parts.
#[derive(Clone, Debug)]
pub struct Part {
    pub points: Vec<[f32; 2]>,
    pub ink: f32,
}

fn part(points: Vec<[f32; 2]>) -> Part {
    Part { points, ink: 1.0 }
}

fn cut(points: Vec<[f32; 2]>) -> Part {
    Part { points, ink: 0.0 }
}

fn circle(cx: f32, cy: f32, r: f32) -> Vec<[f32; 2]> {
    (0..24)
        .map(|i| {
            let a = i as f32 / 24.0 * TAU;
            [cx + a.cos() * r, cy + a.sin() * r]
        })
        .collect()
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<[f32; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn rounded(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Vec<[f32; 2]> {
    let mut points = Vec::new();
    for (cx, cy, start) in [
        (x1 - r, y0 + r, -PI / 2.0),
        (x1 - r, y1 - r, 0.0),
        (x0 + r, y1 - r, PI / 2.0),
        (x0 + r, y0 + r, PI),
    ] {
        for k in 0..=5 {
            let a = start + k as f32 / 5.0 * PI / 2.0;
            points.push([cx + a.cos() * r, cy + a.sin() * r]);
        }
    }
    points
}

/// A straight band of width `w` from `a` to `b`.
fn stroke(a: [f32; 2], b: [f32; 2], w: f32) -> Vec<[f32; 2]> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-4);
    let n = [-d[1] / len * w * 0.5, d[0] / len * w * 0.5];
    vec![
        [a[0] + n[0], a[1] + n[1]],
        [b[0] + n[0], b[1] + n[1]],
        [b[0] - n[0], b[1] - n[1]],
        [a[0] - n[0], a[1] - n[1]],
    ]
}

/// A curved band: the ring between radii `inner` and `outer` from angle
/// `from` to `to` (radians, y down).
fn arc(cx: f32, cy: f32, inner: f32, outer: f32, from: f32, to: f32) -> Vec<[f32; 2]> {
    let steps = 12;
    let mut points: Vec<[f32; 2]> = (0..=steps)
        .map(|k| {
            let a = from + (to - from) * k as f32 / steps as f32;
            [cx + a.cos() * outer, cy + a.sin() * outer]
        })
        .collect();
    points.extend((0..=steps).rev().map(|k| {
        let a = from + (to - from) * k as f32 / steps as f32;
        [cx + a.cos() * inner, cy + a.sin() * inner]
    }));
    points
}

/// A door-like arch: straight sides from `bottom` up to `spring`, then a
/// half circle.
fn arch(x0: f32, x1: f32, spring: f32, bottom: f32) -> Vec<[f32; 2]> {
    let (cx, r) = ((x0 + x1) * 0.5, (x1 - x0) * 0.5);
    let mut points = vec![[x0, bottom]];
    for k in 0..=12 {
        let a = PI + k as f32 / 12.0 * PI;
        points.push([cx + a.cos() * r, spring + a.sin() * r]);
    }
    points.push([x1, bottom]);
    points
}

/// The icon's shape, parts in drawing order.
pub fn shape(icon: Icon) -> Vec<Part> {
    match icon {
        Icon::Door => vec![
            part(arch(-0.27, 0.27, -0.06, 0.42)),
            cut(stroke([0.0, -0.22], [0.0, 0.42], 0.035)),
            cut(circle(0.14, 0.14, 0.045)),
        ],
        Icon::Leave => vec![
            part(rect(-0.44, -0.36, -0.34, 0.36)),
            part(vec![
                [-0.24, -0.09],
                [0.10, -0.09],
                [0.10, -0.26],
                [0.43, 0.0],
                [0.10, 0.26],
                [0.10, 0.09],
                [-0.24, 0.09],
            ]),
        ],
        Icon::Talk => vec![
            part(rounded(-0.43, -0.34, 0.43, 0.18, 0.15)),
            part(vec![[-0.22, 0.12], [0.02, 0.12], [-0.28, 0.42]]),
            cut(circle(-0.2, -0.08, 0.06)),
            cut(circle(0.0, -0.08, 0.06)),
            cut(circle(0.2, -0.08, 0.06)),
        ],
        Icon::Shop => vec![
            part(vec![
                [-0.13, -0.18],
                [0.13, -0.18],
                [0.22, -0.4],
                [-0.22, -0.4],
            ]),
            part(circle(0.0, 0.12, 0.32)),
            cut(rect(-0.16, -0.21, 0.16, -0.15)),
            cut(circle(0.0, 0.14, 0.15)),
            part(circle(0.0, 0.14, 0.09)),
        ],
        Icon::Wardrobe => vec![
            part(arc(0.0, -0.27, 0.05, 0.11, PI, PI * 2.25)),
            part(stroke([0.0, -0.18], [0.0, -0.08], 0.07)),
            part(stroke([0.0, -0.1], [-0.44, 0.24], 0.08)),
            part(stroke([0.0, -0.1], [0.44, 0.24], 0.08)),
            part(stroke([-0.44, 0.24], [0.44, 0.24], 0.08)),
        ],
        Icon::Mine => vec![
            part(arc(0.0, 0.34, 0.46, 0.58, PI * 1.18, PI * 1.82)),
            part(stroke([0.0, -0.2], [0.0, 0.46], 0.1)),
        ],
        Icon::Castle => vec![
            part(vec![
                [-0.38, 0.42],
                [-0.38, -0.36],
                [-0.22, -0.36],
                [-0.22, -0.2],
                [-0.08, -0.2],
                [-0.08, -0.36],
                [0.08, -0.36],
                [0.08, -0.2],
                [0.22, -0.2],
                [0.22, -0.36],
                [0.38, -0.36],
                [0.38, 0.42],
            ]),
            cut(arch(-0.12, 0.12, 0.18, 0.42)),
            cut(rect(-0.05, -0.08, 0.05, 0.06)),
        ],
        Icon::House => vec![
            part(rect(0.16, -0.34, 0.27, -0.08)),
            part(vec![
                [-0.4, 0.0],
                [0.0, -0.38],
                [0.4, 0.0],
                [0.34, 0.0],
                [0.34, 0.42],
                [-0.34, 0.42],
                [-0.34, 0.0],
            ]),
            cut(rect(-0.09, 0.14, 0.09, 0.42)),
            cut(rect(0.14, 0.06, 0.26, 0.18)),
        ],
        Icon::Forest => vec![
            part(rect(-0.05, 0.26, 0.05, 0.44)),
            part(vec![[0.0, -0.44], [0.24, -0.12], [-0.24, -0.12]]),
            part(vec![[0.0, -0.26], [0.32, 0.08], [-0.32, 0.08]]),
            part(vec![[0.0, -0.08], [0.4, 0.3], [-0.4, 0.3]]),
        ],
        Icon::Chest => vec![
            part(rounded(-0.42, -0.34, 0.42, 0.02, 0.14)),
            part(rect(-0.42, -0.02, 0.42, 0.38)),
            cut(rect(-0.42, -0.04, 0.42, 0.02)),
            cut(rect(-0.07, -0.08, 0.07, 0.12)),
        ],
        Icon::Loot => vec![
            part(circle(-0.12, 0.02, 0.28)),
            cut(circle(-0.12, 0.02, 0.18)),
            part(circle(-0.12, 0.02, 0.12)),
            part(circle(0.17, 0.17, 0.25)),
            cut(circle(0.17, 0.17, 0.16)),
            part(circle(0.17, 0.17, 0.1)),
        ],
        Icon::Rune => vec![
            part(vec![[0.0, -0.46], [0.34, 0.0], [0.0, 0.46], [-0.34, 0.0]]),
            cut(vec![[0.0, -0.28], [0.2, 0.0], [0.0, 0.28], [-0.2, 0.0]]),
            part(circle(0.0, 0.0, 0.08)),
        ],
        Icon::Rest => {
            let flame = |s: f32, dy: f32| -> Vec<[f32; 2]> {
                (0..24)
                    .map(|k| {
                        let t = k as f32 / 24.0 * TAU;
                        // A teardrop: round at the base, drawn to a point.
                        let x = t.sin() * (1.0 - t.cos()) * 0.5;
                        let y = -t.cos();
                        [x * 0.36 * s, dy + y * 0.3 * s]
                    })
                    .collect()
            };
            vec![
                part(flame(1.0, -0.06)),
                cut(flame(0.5, 0.06)),
                part(stroke([-0.4, 0.4], [0.4, 0.24], 0.1)),
                part(stroke([-0.4, 0.24], [0.4, 0.4], 0.1)),
            ]
        }
        Icon::Memory => {
            let mut eye: Vec<[f32; 2]> = (0..=12)
                .map(|k| {
                    let a = PI + k as f32 / 12.0 * PI;
                    [
                        a.cos() * 0.44,
                        a.sin() * 0.3 * (1.0 - (a.cos()).abs() * 0.1),
                    ]
                })
                .collect();
            eye.extend((1..12).map(|k| {
                let a = k as f32 / 12.0 * PI;
                [a.cos() * 0.44, a.sin() * 0.3]
            }));
            vec![
                part(eye),
                cut(circle(0.0, 0.0, 0.17)),
                part(circle(0.0, 0.0, 0.08)),
            ]
        }
        Icon::Crack => vec![
            part(rect(-0.42, -0.4, 0.42, 0.4)),
            cut(rect(-0.42, -0.14, 0.42, -0.1)),
            cut(rect(-0.42, 0.12, 0.42, 0.16)),
            cut(stroke([-0.05, -0.4], [0.08, -0.12], 0.07)),
            cut(stroke([0.08, -0.12], [-0.06, 0.12], 0.07)),
            cut(stroke([-0.06, 0.12], [0.1, 0.4], 0.07)),
        ],
        Icon::Lock => vec![
            part(arc(0.0, -0.08, 0.16, 0.26, PI, TAU)),
            part(rounded(-0.34, -0.1, 0.34, 0.42, 0.08)),
            cut(circle(0.0, 0.1, 0.07)),
            cut(rect(-0.03, 0.1, 0.03, 0.28)),
        ],
        Icon::Tavern => vec![
            part(arc(0.2, 0.1, 0.1, 0.19, -PI / 2.0, PI / 2.0)),
            part(rounded(-0.34, -0.18, 0.2, 0.42, 0.06)),
            part(circle(-0.22, -0.2, 0.13)),
            part(circle(-0.05, -0.25, 0.14)),
            part(circle(0.11, -0.2, 0.12)),
            cut(rect(-0.2, -0.02, -0.14, 0.32)),
            cut(rect(-0.04, -0.02, 0.02, 0.32)),
        ],
        Icon::Bath => {
            let steam = |x: f32| -> Vec<Part> {
                (0..3)
                    .map(|k| {
                        let y0 = -0.44 + k as f32 * 0.12;
                        let side = if k % 2 == 0 { 0.05 } else { -0.05 };
                        part(stroke([x - side, y0], [x + side, y0 + 0.12], 0.06))
                    })
                    .collect()
            };
            let mut parts = steam(-0.18);
            parts.extend(steam(0.0));
            parts.extend(steam(0.18));
            let mut basin = vec![[-0.44, 0.02], [0.44, 0.02]];
            basin.extend((1..12).map(|k| {
                let a = k as f32 / 12.0 * PI;
                [a.cos() * 0.44, 0.02 + a.sin() * 0.4]
            }));
            parts.push(part(basin));
            parts.push(cut(stroke([-0.3, 0.14], [0.3, 0.14], 0.05)));
            parts
        }
        Icon::Council => vec![
            part(rect(-0.28, -0.3, 0.28, 0.3)),
            part(rounded(-0.38, -0.42, 0.38, -0.26, 0.08)),
            part(rounded(-0.38, 0.26, 0.38, 0.42, 0.08)),
            cut(rect(-0.18, -0.16, 0.18, -0.11)),
            cut(rect(-0.18, -0.03, 0.18, 0.02)),
            cut(rect(-0.18, 0.1, 0.08, 0.15)),
        ],
        Icon::Heart => vec![
            part(circle(-0.17, -0.1, 0.22)),
            part(circle(0.17, -0.1, 0.22)),
            part(vec![[-0.37, -0.02], [0.37, -0.02], [0.0, 0.42]]),
        ],
        Icon::Play => vec![part(vec![[-0.24, -0.36], [0.38, 0.0], [-0.24, 0.36]])],
        Icon::Disk => vec![
            part(rounded(-0.4, -0.4, 0.4, 0.4, 0.07)),
            cut(rect(-0.24, -0.4, 0.16, -0.14)),
            part(rect(0.02, -0.36, 0.1, -0.18)),
            cut(rect(-0.26, 0.04, 0.26, 0.4)),
            part(rect(-0.18, 0.12, 0.18, 0.18)),
            part(rect(-0.18, 0.26, 0.1, 0.32)),
        ],
        Icon::Folder => vec![
            part(vec![
                [-0.44, -0.3],
                [-0.1, -0.3],
                [0.02, -0.18],
                [0.44, -0.18],
                [0.44, 0.34],
                [-0.44, 0.34],
            ]),
            cut(rect(-0.44, -0.06, 0.44, -0.02)),
        ],
        Icon::Backpack => vec![
            part(arc(0.0, -0.3, 0.07, 0.15, PI, TAU)),
            part(rounded(-0.44, 0.0, -0.3, 0.34, 0.06)),
            part(rounded(0.3, 0.0, 0.44, 0.34, 0.06)),
            part(rounded(-0.32, -0.3, 0.32, 0.44, 0.18)),
            cut(stroke([-0.32, -0.06], [0.32, -0.06], 0.04)),
            cut(rounded(-0.17, 0.08, 0.17, 0.3, 0.05)),
            part(rect(-0.04, -0.02, 0.04, 0.05)),
        ],
        Icon::Gear => {
            let mut points = Vec::new();
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let w = TAU / 8.0;
                for (da, r) in [(-0.3, 0.3), (-0.18, 0.44), (0.18, 0.44), (0.3, 0.3)] {
                    points.push([(a + da * w).cos() * r, (a + da * w).sin() * r]);
                }
            }
            vec![part(points), cut(circle(0.0, 0.0, 0.14))]
        }
        Icon::Power => vec![
            part(arc(
                0.0,
                0.05,
                0.26,
                0.37,
                -PI / 2.0 + 0.75,
                -PI / 2.0 + TAU - 0.75,
            )),
            part(stroke([0.0, -0.42], [0.0, -0.02], 0.13)),
        ],
        Icon::Crown => vec![
            part(vec![
                [-0.42, 0.3],
                [-0.42, -0.24],
                [-0.2, -0.02],
                [0.0, -0.4],
                [0.2, -0.02],
                [0.42, -0.24],
                [0.42, 0.3],
            ]),
            cut(rect(-0.42, 0.14, 0.42, 0.18)),
            cut(circle(0.0, 0.03, 0.05)),
        ],
        Icon::Feast => vec![
            part(rect(-0.34, -0.42, -0.28, -0.06)),
            part(rect(-0.24, -0.42, -0.18, -0.06)),
            part(rect(-0.14, -0.42, -0.08, -0.06)),
            part(rounded(-0.34, -0.1, -0.08, 0.06, 0.05)),
            part(stroke([-0.21, 0.02], [-0.21, 0.44], 0.11)),
            part(vec![
                [0.14, -0.42],
                [0.32, -0.3],
                [0.32, 0.02],
                [0.14, 0.02],
            ]),
            part(stroke([0.23, 0.0], [0.23, 0.44], 0.11)),
        ],
    }
}

/// Triangles filling a simple polygon (either winding), by ear clipping.
pub fn triangulate(points: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let count = points.len();
    if count < 3 {
        return Vec::new();
    }
    let area: f32 = (0..count)
        .map(|i| {
            let [x0, y0] = points[i];
            let [x1, y1] = points[(i + 1) % count];
            x0 * y1 - x1 * y0
        })
        .sum();
    let ccw = area >= 0.0;
    let mut polygon: Vec<usize> = (0..count).collect();
    let mut triangles = Vec::with_capacity(count - 2);
    let mut guard = 0;
    while polygon.len() > 3 && guard < count * count {
        guard += 1;
        let n = polygon.len();
        let mut clipped = false;
        for i in 0..n {
            let (a, b, c) = (polygon[(i + n - 1) % n], polygon[i], polygon[(i + 1) % n]);
            let [ax, ay] = points[a];
            let [bx, by] = points[b];
            let [cx, cy] = points[c];
            let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
            if (ccw && cross <= 1e-7) || (!ccw && cross >= -1e-7) {
                continue;
            }
            let inside = polygon.iter().any(|&p| {
                if p == a || p == b || p == c {
                    return false;
                }
                let [px, py] = points[p];
                let d0 = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
                let d1 = (cx - bx) * (py - by) - (cy - by) * (px - bx);
                let d2 = (ax - cx) * (py - cy) - (ay - cy) * (px - cx);
                let negative = d0 < 0.0 || d1 < 0.0 || d2 < 0.0;
                let positive = d0 > 0.0 || d1 > 0.0 || d2 > 0.0;
                !(negative && positive)
            });
            if inside {
                continue;
            }
            triangles.push([a, b, c]);
            polygon.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Degenerate leftovers: fan what remains rather than lose it.
            break;
        }
    }
    for i in 1..polygon.len().saturating_sub(1) {
        triangles.push([polygon[0], polygon[i], polygon[i + 1]]);
    }
    triangles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[[f32; 2]]) -> f32 {
        (0..points.len())
            .map(|i| {
                let [x0, y0] = points[i];
                let [x1, y1] = points[(i + 1) % points.len()];
                x0 * y1 - x1 * y0
            })
            .sum::<f32>()
            .abs()
            * 0.5
    }

    #[test]
    fn every_icon_fits_its_box_and_fills_exactly() {
        for icon in Icon::ALL {
            let parts = shape(icon);
            assert!(!parts.is_empty(), "{icon:?}");
            for part in &parts {
                assert!(part.points.len() >= 3, "{icon:?}");
                assert!(
                    part.points
                        .iter()
                        .flatten()
                        .all(|v| v.is_finite() && v.abs() <= 0.5),
                    "{icon:?} leaves its box"
                );
                let tris = triangulate(&part.points);
                let filled: f32 = tris
                    .iter()
                    .map(|t| area(&[part.points[t[0]], part.points[t[1]], part.points[t[2]]]))
                    .sum();
                let expected = area(&part.points);
                assert!(
                    (filled - expected).abs() <= expected * 0.02 + 1e-4,
                    "{icon:?}: filled {filled} of {expected}"
                );
            }
        }
    }

    #[test]
    fn icons_have_distinct_colours() {
        for (i, a) in Icon::ALL.iter().enumerate() {
            assert_eq!(a.index(), i);
        }
        let colours: std::collections::BTreeSet<[u32; 3]> = Icon::ALL
            .iter()
            .map(|i| i.accent().map(|c| (c * 100.0) as u32))
            .collect();
        assert_eq!(colours.len(), Icon::ALL.len());
    }

    /// `ICON_SHEET=path cargo test ... -- --ignored`: dump every icon as
    /// JSON polygons for a visual review.
    #[test]
    #[ignore]
    fn dump_icon_sheet() {
        let Some(path) = std::env::var_os("ICON_SHEET") else {
            return;
        };
        let sheet: Vec<String> = Icon::ALL
            .iter()
            .map(|&icon| {
                let parts: Vec<String> = shape(icon)
                    .iter()
                    .map(|p| format!("{{\"ink\":{},\"points\":{:?}}}", p.ink, p.points))
                    .collect();
                format!(
                    "{{\"name\":\"{icon:?}\",\"accent\":{:?},\"parts\":[{}]}}",
                    icon.accent(),
                    parts.join(",")
                )
            })
            .collect();
        std::fs::write(path, format!("[{}]", sheet.join(","))).unwrap();
    }

    #[test]
    fn prompt_text_names_the_key_only_when_it_acts() {
        let open = Prompt::new(Icon::Chest, "Abrir").detail("Relicario");
        assert_eq!(open.text(), "F  Abrir · Relicario");
        let sealed = Prompt::new(Icon::Lock, "Sellado").locked();
        assert_eq!(sealed.text(), "Sellado");
    }
}
