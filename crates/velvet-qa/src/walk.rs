//! The walker's ground: what a navmesh says about a zone, where to walk, and how to follow a
//! way there.
use std::collections::VecDeque;
use velvet_nav::NavMesh;

/// What a zone's navmesh says about it, seen from where the zone starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Survey {
    /// Which of the navmesh's rects can be reached from the start.
    pub reach: Vec<bool>,
    /// All the walkable ground (m²).
    pub walkable_m2: f32,
    /// The ground reachable from the start (m²).
    pub reachable_m2: f32,
    /// Ground cut off from the start that reaches the zone's edge: the open space round a
    /// building (the collision has walls, not floors). Noted, not an island.
    pub outside_m2: f32,
    /// Ground of a square metre or more cut off from the start: a point in it, and its area.
    pub islands: Vec<([f32; 2], f32)>,
}

impl Survey {
    /// Surveys `nav` from `start`.
    pub fn of(nav: &NavMesh, start: [f32; 2]) -> Self {
        let reach = nav.reachable_from(start[0], start[1]);
        let mut survey = Self {
            reach: reach.clone(),
            ..Self::default()
        };
        for r in 0..nav.rects.len() as u32 {
            let a = nav.rect_area(r);
            survey.walkable_m2 += a;
            if reach[r as usize] {
                survey.reachable_m2 += a;
            }
        }
        let mut seen = reach;
        for r in 0..nav.rects.len() as u32 {
            if seen[r as usize] {
                continue;
            }
            let c = nav.rect_center(r);
            let island = nav.reachable_from(c[0], c[1]);
            let area: f32 = (0..nav.rects.len() as u32)
                .filter(|&k| island[k as usize])
                .map(|k| nav.rect_area(k))
                .sum();
            for (k, v) in island.iter().enumerate() {
                if *v {
                    seen[k] = true;
                }
            }
            if island
                .iter()
                .enumerate()
                .any(|(k, v)| *v && nav.rect_on_border(k as u32))
            {
                survey.outside_m2 += area;
                continue;
            }
            if area >= 1.0 {
                survey.islands.push((c, area));
            }
        }
        survey
    }
}

/// Up to `count` places to walk to over the reachable ground (`reach`, as [`Survey::reach`]),
/// each as far as can be from the start and from those picked before it. Rects smaller than
/// `min_area` m² are left out.
pub fn spread_targets(
    nav: &NavMesh,
    reach: &[bool],
    start: [f32; 2],
    count: usize,
    min_area: f32,
) -> Vec<[f32; 2]> {
    let mut candidates: Vec<[f32; 2]> = (0..nav.rects.len() as u32)
        .filter(|&r| reach[r as usize] && nav.rect_area(r) >= min_area)
        .map(|r| nav.rect_center(r))
        .collect();
    let mut targets = Vec::new();
    let mut chosen = vec![start];
    while targets.len() < count && !candidates.is_empty() {
        let (k, _) = candidates
            .iter()
            .enumerate()
            .map(|(k, c)| {
                (
                    k,
                    chosen
                        .iter()
                        .map(|q| (q[0] - c[0]).hypot(q[1] - c[1]))
                        .fold(f32::INFINITY, f32::min),
                )
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .expect("there are candidates");
        let c = candidates.swap_remove(k);
        chosen.push(c);
        targets.push(c);
    }
    targets
}

/// How a walk is going after a step along its way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Getting along: carry on.
    Going,
    /// Not getting anywhere for two seconds: a fresh way from here was found; try it at once.
    Replanned,
    /// Still not getting anywhere: stuck.
    Stuck,
}

/// Walking a navmesh way corner by corner, as a player holding the keys would: each step, ask
/// for the [`corner`](Self::corner) to head for, take the step, then tell the follower how it
/// went ([`progress`](Self::progress)) until it is reached, stuck or out of time.
#[derive(Clone, Debug)]
pub struct PathFollower {
    path: VecDeque<[f32; 2]>,
    target: [f32; 2],
    /// The time allowed (s): a slow walk there, and some for each corner.
    limit: f32,
    t: f32,
    /// The least that was ever left of the way (m), and how long since it last shrank.
    best: f32,
    since_progress: f32,
}

impl PathFollower {
    /// The way from `from` to `target`, if the navmesh has one.
    pub fn new(nav: &NavMesh, from: [f32; 2], target: [f32; 2]) -> Option<Self> {
        let path = nav.find_path(from, target)?;
        let total = (target[0] - from[0]).hypot(target[1] - from[1]);
        let limit = 4.0 + total * 2.5 / 2.2 + 2.0 * path.len() as f32;
        Some(Self {
            path: VecDeque::from(path),
            target,
            limit,
            t: 0.0,
            best: f32::INFINITY,
            since_progress: 0.0,
        })
    }

    /// Whether there is still time to get there.
    pub fn in_time(&self) -> bool {
        self.t < self.limit
    }

    /// The corner to head for from `p`, dropping those reached and those passed when the one
    /// after is in plain sight; `None` once the way is walked (the walker is there).
    pub fn corner(&mut self, nav: &NavMesh, p: [f32; 2]) -> Option<[f32; 2]> {
        while let Some(&c) = self.path.front() {
            let near = (c[0] - p[0]).hypot(c[1] - p[1]);
            let next_clear = self.path.len() > 1 && near < 1.0 && nav.clear_line(p, self.path[1]);
            if (self.path.len() > 1 && (near < 0.15 || next_clear))
                || (self.path.len() == 1 && near < 0.3)
            {
                self.path.pop_front();
            } else {
                break;
            }
        }
        self.path.front().copied()
    }

    /// After a step of `dt` seconds taken from `p` toward `corner`: whether the walker is still
    /// getting somewhere. Progress is what is left of the way (it may lead away from the target
    /// first); two seconds without any earn one fresh way from `p`, and then it is stuck.
    pub fn progress(&mut self, nav: &NavMesh, p: [f32; 2], corner: [f32; 2], dt: f32) -> Progress {
        let mut d = (corner[0] - p[0]).hypot(corner[1] - p[1]);
        for w in self.path.iter().collect::<Vec<_>>().windows(2) {
            d += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        }
        if d < self.best - 0.05 {
            self.best = d;
            self.since_progress = 0.0;
        } else {
            self.since_progress += dt;
        }
        if self.since_progress > 2.0 {
            if let Some(fresh) = nav.find_path(p, self.target) {
                if self.since_progress < 2.05 {
                    self.path = VecDeque::from(fresh);
                    return Progress::Replanned;
                }
            }
            return Progress::Stuck;
        }
        self.t += dt;
        Progress::Going
    }

    /// The least that was left of the way (metres): how far short a stuck walker got.
    pub fn short_by(&self) -> f32 {
        self.best
    }
}

/// The keys (forward, back, left, right) to hold to walk closest to `dir` (a unit vector on the
/// ground) in a view whose forward and right are `forward` and `right`: the best of the eight
/// directions a keyboard walks in.
pub fn eight_way_keys(
    forward: [f32; 2],
    right: [f32; 2],
    dir: [f32; 2],
) -> (bool, bool, bool, bool) {
    let (f, r, d) = (forward, right, dir);
    let mut best = (f32::NEG_INFINITY, (false, false, false, false));
    for (fw, bk, lf, rt) in [
        (true, false, false, false),
        (false, true, false, false),
        (false, false, true, false),
        (false, false, false, true),
        (true, false, true, false),
        (true, false, false, true),
        (false, true, true, false),
        (false, true, false, true),
    ] {
        let mut v = [0.0f32; 2];
        if fw {
            v = [v[0] + f[0], v[1] + f[1]];
        }
        if bk {
            v = [v[0] - f[0], v[1] - f[1]];
        }
        if lf {
            v = [v[0] - r[0], v[1] - r[1]];
        }
        if rt {
            v = [v[0] + r[0], v[1] + r[1]];
        }
        let l = v[0].hypot(v[1]);
        let score = (v[0] * d[0] + v[1] * d[1]) / l;
        if score > best.0 {
            best = (score, (fw, bk, lf, rt));
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two rooms 6 m square joined by a door 1 m wide, and a closet nobody can get into.
    fn rooms() -> NavMesh {
        NavMesh::build([0.0, 0.0, 14.0, 6.0], 0.25, 10.0, |x, z| {
            let wall = (6.0..7.0).contains(&x) && !(2.5..3.5).contains(&z);
            let closet_walls = (10.0..14.0).contains(&x)
                && (0.0..3.0).contains(&z)
                && !((10.5..13.5).contains(&x) && (0.5..2.5).contains(&z));
            (!wall && !closet_walls).then_some(0.0)
        })
    }

    #[test]
    fn a_survey_finds_the_ground_and_what_is_cut_off() {
        let nav = rooms();
        let survey = Survey::of(&nav, [1.0, 1.0]);
        assert!(survey.walkable_m2 > 70.0, "{}", survey.walkable_m2);
        assert!(survey.reachable_m2 > 60.0 && survey.reachable_m2 < survey.walkable_m2);
        assert_eq!(survey.islands.len(), 1, "{:?}", survey.islands);
        let (at, area) = survey.islands[0];
        assert!(
            (10.5..13.5).contains(&at[0])
                && (0.5..2.5).contains(&at[1])
                && (area - 6.0).abs() < 0.6,
            "{at:?} {area}"
        );
        assert_eq!(
            survey.outside_m2, 0.0,
            "the closet keeps off the zone's edge"
        );
    }

    #[test]
    fn targets_spread_from_the_start_over_reachable_ground() {
        let nav = rooms();
        let survey = Survey::of(&nav, [1.0, 1.0]);
        let targets = spread_targets(&nav, &survey.reach, [1.0, 1.0], 100, 0.25);
        let candidates = (0..nav.rects.len() as u32)
            .filter(|&r| survey.reach[r as usize] && nav.rect_area(r) >= 0.25)
            .count();
        assert_eq!(
            targets.len(),
            candidates,
            "every candidate once when more are asked for"
        );
        assert_eq!(
            spread_targets(&nav, &survey.reach, [1.0, 1.0], 2, 0.25),
            targets[..2]
        );
        // The first is the farthest from the start: in the second room.
        assert!(targets[0][0] > 7.0, "{:?}", targets[0]);
        for (k, t) in targets.iter().enumerate() {
            assert!(!targets[..k].contains(t), "each once: {t:?}");
            assert!(nav.walkable_at(t[0], t[1]));
            assert!(
                !((10.5..13.5).contains(&t[0]) && (0.5..2.5).contains(&t[1])),
                "never in the closet: {t:?}"
            );
        }
    }

    #[test]
    fn a_follower_walks_through_the_door_and_knows_when_it_is_stuck() {
        let nav = rooms();
        let dt: f32 = 1.0 / 60.0;
        // Walking at 2 m/s straight at each corner: it gets there.
        let mut follow = PathFollower::new(&nav, [1.0, 1.0], [12.0, 5.0]).expect("a way");
        let mut p = [1.0f32, 1.0];
        let mut arrived = false;
        while follow.in_time() {
            let Some(c) = follow.corner(&nav, p) else {
                arrived = true;
                break;
            };
            let (dx, dz) = (c[0] - p[0], c[1] - p[1]);
            let len = dx.hypot(dz).max(1e-6);
            let step = (2.0 * dt).min(len);
            let before = p;
            p = [p[0] + dx / len * step, p[1] + dz / len * step];
            assert_eq!(follow.progress(&nav, before, c, dt), Progress::Going);
        }
        assert!(arrived, "at {p:?}");
        // A walker that never moves: two seconds, one fresh way, then stuck.
        let mut follow = PathFollower::new(&nav, [1.0, 1.0], [12.0, 5.0]).unwrap();
        let mut outcomes = Vec::new();
        for _ in 0..400 {
            let c = follow.corner(&nav, [1.0, 1.0]).unwrap();
            let o = follow.progress(&nav, [1.0, 1.0], c, dt);
            outcomes.push(o);
            if o == Progress::Stuck {
                break;
            }
        }
        assert_eq!(outcomes.last(), Some(&Progress::Stuck));
        assert!(outcomes.contains(&Progress::Replanned));
        assert!(
            outcomes.len() > 120 && outcomes.len() < 130,
            "{}",
            outcomes.len()
        );
        assert!(follow.short_by() > 10.0);
        // No way at all: no follower.
        assert!(PathFollower::new(&nav, [1.0, 1.0], [12.0, 1.5]).is_none());
    }

    #[test]
    fn the_keys_walk_closest_to_the_direction() {
        let (f, r) = ([0.0, -1.0], [1.0, 0.0]);
        assert_eq!(
            eight_way_keys(f, r, [0.0, -1.0]),
            (true, false, false, false)
        );
        assert_eq!(
            eight_way_keys(f, r, [-1.0, 0.0]),
            (false, false, true, false)
        );
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(eight_way_keys(f, r, [s, s]), (false, true, false, true));
        assert_eq!(
            eight_way_keys(f, r, [0.3, -0.95]),
            (true, false, false, false)
        );
    }
}
