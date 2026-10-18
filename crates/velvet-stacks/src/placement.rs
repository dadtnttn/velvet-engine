//! Output placement: where the copies that a finished job makes appear.
//!
//! The default, [`OutputPlacement::Row`], is the rule of version 1: the copies make a row to the right
//! of the stack, [`OUTPUT_STEP`] apart. [`OutputPlacement::Free`] puts each copy on the nearest free
//! spot of a grid, inside optional bounds, or anywhere on the table when the bounds hold none. The
//! placement is configuration, like the catalog: a save does not hold it, and a game sets it again
//! after it loads a board.
//!
//! The free search is the rule of `findFreeSpot` in `js/packages/cards/src/spot.ts`, computed in
//! binary32 operation for operation, so the two pick the same spot for the same table. The cross-check
//! in `js/packages/cards/src/board.test.ts` holds them to it.

use serde::{Deserialize, Serialize};

use crate::board::Board;
use crate::error::StacksError;
use crate::job::OUTPUT_STEP;

/// A rectangle `(x, y, width, height)`, in board units.
type Rect = (f32, f32, f32, f32);

/// The most rings the search looks at without bounds: the constant `MAX_FREE_RING` of
/// `docs/formats/STACKS.md`. A stack blocks at most four spots, so the rings up to this one hold a free
/// spot for any table of fewer than about a million stacks, and an ordinary table finds its nearest free
/// spot well inside them. The cap also keeps the search finite for footprints so small that its stopping
/// rule underflows in binary32 (below about 3e-23 board units), where the search would otherwise run on.
const MAX_FREE_RING: i64 = 1000;

/// Where the copies that a finished job makes appear. Set with [`Board::set_output_placement`].
///
/// It is written as the placement document of `docs/formats/STACKS.md`: `{"kind": "row"}`, or
/// `{"kind": "free", "footprint": [w, h], "bounds": [x, y, w, h]}`, where `bounds` is left out for
/// the whole table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutputPlacement {
    /// The copies make a row to the right of the stack, [`OUTPUT_STEP`] board units apart, in the
    /// order the recipe lists them. This is the rule of version 1, and the default.
    #[default]
    Row,
    /// Each copy lands on the nearest free spot of a grid. The grid steps by `footprint` and starts
    /// one footprint width to the right of the stack. A spot is free when no stack overlaps it and,
    /// when `bounds` is given, it lies inside them. When no spot inside the bounds is free, the copy
    /// takes the nearest free spot anywhere on the table. Only when there is none does it land in its
    /// row position, even when a stack is already there.
    Free {
        /// The size of a stack, `[width, height]`: both sides finite and above zero. Every stack
        /// covers this size from its position.
        footprint: (f32, f32),
        /// The rectangle `[x, y, width, height]` that a spot must lie in, edges included: finite
        /// numbers, with a width and a height of zero or more. `None` means the whole table.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bounds: Option<(f32, f32, f32, f32)>,
    },
}

impl OutputPlacement {
    /// Refuses a free placement whose numbers cannot hold a stack. The row placement has none to check.
    pub(crate) fn check(&self) -> Result<(), StacksError> {
        let OutputPlacement::Free { footprint, bounds } = *self else {
            return Ok(());
        };
        let (width, height) = footprint;
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            return Err(StacksError::BadPlacement(format!(
                "a footprint needs two finite sides above zero, not ({width}, {height})"
            )));
        }
        if let Some((x, y, w, h)) = bounds {
            // The far edges must be finite too, so that the search never works with infinities.
            let finite = [x, y, w, h].iter().all(|v| v.is_finite())
                && (x + w).is_finite()
                && (y + h).is_finite();
            if !finite || w < 0.0 || h < 0.0 {
                return Err(StacksError::BadPlacement(format!(
                    "bounds need finite numbers and a width and height of zero or more, not ({x}, {y}, {w}, {h})"
                )));
            }
        }
        Ok(())
    }
}

impl Board {
    /// Sets where the copies of finished jobs appear from now on. The placement is checked first: a
    /// refused one fails with [`StacksError::BadPlacement`] and the placement in use stays. Queues no
    /// events, and changes no card, stack or job.
    ///
    /// The placement is configuration, not state. [`Board::save`] does not write it, and a board made by
    /// [`Board::load`] starts with [`OutputPlacement::Row`], so a game sets it again after loading.
    pub fn set_output_placement(&mut self, placement: OutputPlacement) -> Result<(), StacksError> {
        placement.check()?;
        self.output_placement = placement;
        Ok(())
    }

    /// The placement that finished jobs use now: [`OutputPlacement::Row`] unless a game set another.
    pub fn output_placement(&self) -> OutputPlacement {
        self.output_placement
    }

    /// Where copy number `made` of a finish appears, given the position `origin` of the stack before
    /// its cards were used up. `made` counts the copies this finish has already made, from 0.
    ///
    /// With the row placement the copy goes to `origin.x + OUTPUT_STEP * (made + 1)`, `origin.y`. With
    /// the free placement it goes to the nearest free spot from `(origin.x + footprint width, origin.y)`,
    /// judged against every stack on the table now, copies made earlier in this finish included. When no
    /// spot inside the bounds is free, the search runs again without bounds. Only when that finds none
    /// either, the copy falls back to its row position, which may overlap a stack.
    pub(crate) fn output_spot(&self, origin: (f32, f32), made: usize) -> (f32, f32) {
        let row = (origin.0 + OUTPUT_STEP * (made as f32 + 1.0), origin.1);
        match self.output_placement {
            OutputPlacement::Row => row,
            OutputPlacement::Free { footprint, bounds } => {
                let near = (origin.0 + footprint.0, origin.1);
                let obstacles: Vec<(f32, f32)> = self
                    .stacks
                    .keys()
                    .filter_map(|&stack| self.stack_position(stack))
                    .collect();
                let inside = free_spot(&obstacles, near, footprint, bounds);
                let spot = if bounds.is_some() && inside.is_none() {
                    free_spot(&obstacles, near, footprint, None)
                } else {
                    inside
                };
                spot.unwrap_or(row)
            }
        }
    }
}

/// The nearest free spot, or `None` when the rings searched hold none. With bounds that happens when
/// every spot inside them is taken. Without bounds it does not happen for a table of fewer than about a
/// million stacks, unless binary32 cannot tell neighbouring spots apart (see [`Search::ring_range`]). It is
/// the search of `findFreeSpot` in `spot.ts`, in binary32, and it returns the same spot for the same table.
///
/// `obstacles` are the positions of the stacks on the table, `near` is where the search starts,
/// `size` is the footprint, and `bounds` is the rectangle the spot must lie in, if any.
///
/// The spots are `near + (i * width, j * height)` for whole numbers `i` and `j`. Spot (i, j) is in ring
/// `max(|i|, |j|)`, and its distance is `dx * dx + dy * dy`, where `dx = i * width` and `dy = j * height`.
/// The search keeps the candidate with the least `(distance, j, i)`: the nearest spot, then the smaller
/// offset in y, then the smaller offset in x. It looks at the rings from the first to the last one
/// listed for the table (see [`Search::ring_range`]), and stops early once no spot of a further ring
/// can be nearer. The cost is the number of spots tried times the number of stacks.
pub(crate) fn free_spot(
    obstacles: &[(f32, f32)],
    near: (f32, f32),
    size: (f32, f32),
    bounds: Option<Rect>,
) -> Option<(f32, f32)> {
    let (width, height) = size;
    let mut search = Search {
        obstacles,
        near,
        size,
        bounds,
        // Stacks that only touch must not overlap. The tolerance keeps them apart after rounding.
        tolerance: 1e-5_f32 * width.max(height),
        best: None,
    };
    let (first, last) = search.ring_range();
    let smallest_step = width.min(height);
    for ring in first..=last {
        // Every spot of this ring, and of the rings after it, lies at least this far away.
        let reach = ring as f32 * smallest_step;
        if search
            .best
            .is_some_and(|best| reach * reach > best.distance)
        {
            break;
        }
        search.offer_ring(ring);
    }
    search.best.map(|best| best.spot)
}

/// A candidate spot: its squared distance, its grid offsets, and its position.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    distance: f32,
    j: i64,
    i: i64,
    spot: (f32, f32),
}

/// One search over one table: the inputs of the rule and the best candidate so far.
struct Search<'a> {
    obstacles: &'a [(f32, f32)],
    near: (f32, f32),
    size: (f32, f32),
    bounds: Option<Rect>,
    tolerance: f32,
    best: Option<Candidate>,
}

impl Search<'_> {
    /// The first and last ring to search. Rings outside them hold no candidate: with bounds, the
    /// rings start one short of the bounds' nearest point and end one past their far corners; without
    /// bounds, they end two past the farthest stack. Steps count the larger axis, in units of the
    /// footprint, and a spot's ring is its number of steps. Without bounds the last ring is capped at
    /// [`MAX_FREE_RING`], which keeps the search finite.
    fn ring_range(&self) -> (i64, i64) {
        let (width, height) = self.size;
        let (near_x, near_y) = self.near;
        let steps = |x: f32, y: f32| ((x - near_x).abs() / width).max((y - near_y).abs() / height);
        match self.bounds {
            None => {
                let farthest = self
                    .obstacles
                    .iter()
                    .fold(0.0_f32, |far, &(x, y)| far.max(steps(x, y)));
                (
                    0,
                    whole_steps(farthest).saturating_add(2).min(MAX_FREE_RING),
                )
            }
            Some((x, y, w, h)) => {
                // The point of the bounds nearest to `near`, and how many steps away it is.
                let nearest_x = near_x.max(x).min(x + w);
                let nearest_y = near_y.max(y).min(y + h);
                let gap =
                    ((nearest_x - near_x).abs() / width).max((nearest_y - near_y).abs() / height);
                let first = (gap.floor() as i64).saturating_sub(1).max(0);
                let corners = [(x, y), (x + w, y), (x, y + h), (x + w, y + h)];
                let farthest = corners
                    .iter()
                    .fold(0.0_f32, |far, &(cx, cy)| far.max(steps(cx, cy)));
                (first, whole_steps(farthest).saturating_add(1))
            }
        }
    }

    /// Whether a stack at `spot` would overlap an obstacle: the two differ by less than the size on both
    /// axes, less the tolerance. Stacks that only touch do not overlap.
    fn overlaps(&self, spot: (f32, f32)) -> bool {
        let (width, height) = self.size;
        let clear_x = width - self.tolerance;
        let clear_y = height - self.tolerance;
        self.obstacles
            .iter()
            .any(|&(x, y)| (x - spot.0).abs() < clear_x && (y - spot.1).abs() < clear_y)
    }

    /// Whether `spot` lies inside the bounds, edges included. Without bounds every spot does.
    fn inside(&self, spot: (f32, f32)) -> bool {
        match self.bounds {
            None => true,
            Some((x, y, w, h)) => spot.0 >= x && spot.0 <= x + w && spot.1 >= y && spot.1 <= y + h,
        }
    }

    /// Offers every spot of ring `ring`: the offsets whose larger absolute value is `ring`.
    fn offer_ring(&mut self, ring: i64) {
        if ring == 0 {
            self.offer(0, 0);
            return;
        }
        for i in -ring..=ring {
            self.offer(i, -ring);
            self.offer(i, ring);
        }
        for j in (-ring + 1)..ring {
            self.offer(-ring, j);
            self.offer(ring, j);
        }
    }

    /// Keeps the spot at offset (`i`, `j`) when it is a candidate and beats the best so far: a smaller
    /// distance, or the same distance with a smaller `j`, then a smaller `i`.
    fn offer(&mut self, i: i64, j: i64) {
        let (width, height) = self.size;
        let dx = i as f32 * width;
        let dy = j as f32 * height;
        let distance = dx * dx + dy * dy;
        if let Some(best) = self.best {
            let tie = distance == best.distance && (j < best.j || (j == best.j && i < best.i));
            let beats = distance < best.distance || tie;
            if !beats {
                return;
            }
        }
        let spot = (self.near.0 + dx, self.near.1 + dy);
        if self.inside(spot) && !self.overlaps(spot) {
            self.best = Some(Candidate {
                distance,
                j,
                i,
                spot,
            });
        }
    }
}

/// The number of whole steps up to `steps`, rounded up. A value past the range of an `i64` saturates.
fn whole_steps(steps: f32) -> i64 {
    steps.ceil() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::catalog;
    use crate::BoardEvent;

    /// A small deterministic generator of values in `[0, 1)`, so that the random tables repeat.
    fn unit(state: &mut u64) -> f32 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 40) as f32 / (1u64 << 24) as f32
    }

    #[test]
    fn the_start_point_is_taken_when_nothing_is_in_the_way() {
        assert_eq!(
            free_spot(&[], (2.0, 3.0), (1.0, 1.0), None),
            Some((2.0, 3.0))
        );
    }

    #[test]
    fn stacks_that_only_touch_do_not_overlap() {
        assert_eq!(
            free_spot(&[(1.0, 0.0)], (0.0, 0.0), (1.0, 1.0), None),
            Some((0.0, 0.0))
        );
        assert_eq!(
            free_spot(&[(0.0, 1.0)], (0.0, 0.0), (1.0, 1.0), None),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn ties_go_to_the_smaller_y_then_the_smaller_x() {
        // The four spots one step away are all free: (0, -1) has the smallest y.
        assert_eq!(
            free_spot(&[(0.0, 0.0)], (0.0, 0.0), (1.0, 1.0), None),
            Some((0.0, -1.0))
        );
    }

    #[test]
    fn a_stack_that_covers_a_spot_only_partly_blocks_it() {
        assert_eq!(
            free_spot(&[(0.5, 0.0)], (0.0, 0.0), (1.0, 1.0), None),
            Some((0.0, -1.0))
        );
    }

    #[test]
    fn each_side_of_the_footprint_is_its_own_step() {
        // The steps are 2 across and 1 down: (2, 0) is free too, but (0, -1) is nearer.
        assert_eq!(
            free_spot(&[(0.0, 0.0)], (0.0, 0.0), (2.0, 1.0), None),
            Some((0.0, -1.0))
        );
    }

    #[test]
    fn bounds_include_their_edges_and_the_search_starts_from_the_nearest_point() {
        let bounds = Some((0.0, 0.0, 10.0, 10.0));
        assert_eq!(
            free_spot(&[], (-5.0, -5.0), (1.0, 1.0), bounds),
            Some((0.0, 0.0))
        );
        assert_eq!(
            free_spot(&[], (10.0, 10.0), (1.0, 1.0), bounds),
            Some((10.0, 10.0))
        );
        assert_eq!(
            free_spot(&[(10.0, 10.0)], (10.0, 10.0), (1.0, 1.0), bounds),
            Some((10.0, 9.0))
        );
    }

    #[test]
    fn empty_bounds_hold_one_spot_or_none() {
        let bounds = Some((0.0, 0.0, 0.0, 0.0));
        assert_eq!(
            free_spot(&[(0.0, 0.0)], (0.0, 0.0), (1.0, 1.0), bounds),
            None
        );
        // The only spot inside the bounds is five steps from the starting point.
        assert_eq!(
            free_spot(&[], (5.0, 5.0), (1.0, 1.0), bounds),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn the_search_finds_the_same_spot_as_a_scan_of_every_spot() {
        let mut state = 7u64;
        for table in 0..40 {
            let size = (1.0 + unit(&mut state), 0.5 + unit(&mut state));
            let near = (unit(&mut state) * 4.0 - 2.0, unit(&mut state) * 4.0 - 2.0);
            // Some stacks sit on the grid, so that they touch the spots next to them; the others anywhere.
            let count = 1 + (unit(&mut state) * 25.0) as usize;
            let obstacles: Vec<(f32, f32)> = (0..count)
                .map(|_| {
                    if unit(&mut state) < 0.5 {
                        let i = (unit(&mut state) * 11.0).floor() - 5.0;
                        let j = (unit(&mut state) * 11.0).floor() - 5.0;
                        (near.0 + i * size.0, near.1 + j * size.1)
                    } else {
                        (unit(&mut state) * 12.0 - 6.0, unit(&mut state) * 12.0 - 6.0)
                    }
                })
                .collect();

            // The scan: every spot in a window, the free ones only, least by (distance, j, i). It uses the
            // same binary32 steps as the search, so that the two can be compared exactly.
            let tolerance = 1e-5_f32 * size.0.max(size.1);
            let mut expected: Option<(f32, f32)> = None;
            let mut best_key: Option<(f32, i64, i64)> = None;
            for j in -40..=40i64 {
                for i in -40..=40i64 {
                    let dx = i as f32 * size.0;
                    let dy = j as f32 * size.1;
                    let spot = (near.0 + dx, near.1 + dy);
                    let overlapping = obstacles.iter().any(|&(x, y)| {
                        (x - spot.0).abs() < size.0 - tolerance
                            && (y - spot.1).abs() < size.1 - tolerance
                    });
                    if overlapping {
                        continue;
                    }
                    let key = (dx * dx + dy * dy, j, i);
                    let better = best_key.is_none_or(|(d, bj, bi)| {
                        key.0 < d || (key.0 == d && (key.1 < bj || (key.1 == bj && key.2 < bi)))
                    });
                    if better {
                        best_key = Some(key);
                        expected = Some(spot);
                    }
                }
            }
            assert_eq!(
                free_spot(&obstacles, near, size, None),
                expected,
                "table {table}"
            );
        }
    }

    #[test]
    fn the_placement_is_row_by_default_and_a_save_does_not_hold_it() {
        let mut board = Board::new(catalog(), 1);
        assert_eq!(board.output_placement(), OutputPlacement::Row);
        board
            .set_output_placement(OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: None,
            })
            .expect("a valid placement");
        let json = board.to_json().expect("writes");
        assert!(!json.contains("footprint"), "{json}");
        let restored = Board::from_json(catalog(), &json).expect("reads");
        assert_eq!(restored.output_placement(), OutputPlacement::Row);
    }

    #[test]
    fn a_refused_placement_changes_nothing() {
        let good = OutputPlacement::Free {
            footprint: (1.0, 1.0),
            bounds: Some((0.0, 0.0, 4.0, 4.0)),
        };
        let mut board = Board::new(catalog(), 1);
        board.set_output_placement(good).expect("a valid placement");
        let refused = [
            OutputPlacement::Free {
                footprint: (0.0, 1.0),
                bounds: None,
            },
            OutputPlacement::Free {
                footprint: (f32::NAN, 1.0),
                bounds: None,
            },
            OutputPlacement::Free {
                footprint: (1.0, f32::INFINITY),
                bounds: None,
            },
            OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: Some((0.0, 0.0, -1.0, 2.0)),
            },
            OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: Some((0.0, 0.0, f32::INFINITY, 1.0)),
            },
            OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: Some((f32::MAX, 0.0, f32::MAX, 1.0)),
            },
        ];
        for bad in refused {
            assert!(
                matches!(
                    board.set_output_placement(bad),
                    Err(StacksError::BadPlacement(_))
                ),
                "{bad:?}"
            );
            assert_eq!(
                board.output_placement(),
                good,
                "{bad:?} must not replace the placement"
            );
        }
        // Zero-sized bounds are allowed: they hold one spot.
        board
            .set_output_placement(OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: Some((0.0, 0.0, 0.0, 0.0)),
            })
            .expect("zero-sized bounds are a valid placement");
    }

    #[test]
    fn free_copies_land_on_the_nearest_free_spots_one_at_a_time() {
        // A berry bush with a worker makes two berries in one finish (the bonus recipe). The second
        // copy sees the first, so it goes to the next spot.
        let mut board = Board::new(catalog(), 1);
        let bush = board.spawn("berry_bush", (0.0, 0.0)).expect("bush");
        let worker = board.spawn("worker", (0.0, 0.0)).expect("worker");
        board.drop_on(worker, bush).expect("drop");
        board
            .set_output_placement(OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: None,
            })
            .expect("a valid placement");
        board.take_events();
        assert_eq!(
            board.tick(2.0),
            vec![
                BoardEvent::JobFinished {
                    stack: 1,
                    recipe: "pick_berries_bonus".into()
                },
                BoardEvent::CardSpawned {
                    card: 3,
                    def: "berries".into(),
                    x: 1.0,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 3,
                    cards: vec![3]
                },
                BoardEvent::CardSpawned {
                    card: 4,
                    def: "berries".into(),
                    x: 1.0,
                    y: -1.0
                },
                BoardEvent::StackChanged {
                    stack: 4,
                    cards: vec![4]
                },
                BoardEvent::JobStarted {
                    stack: 1,
                    recipe: "pick_berries_bonus".into()
                },
            ]
        );
    }

    #[test]
    fn copies_with_full_bounds_go_to_the_nearest_free_spot_anywhere() {
        let mut board = Board::new(catalog(), 1);
        let bush = board.spawn("berry_bush", (0.0, 0.0)).expect("bush");
        let worker = board.spawn("worker", (0.0, 0.0)).expect("worker");
        board.drop_on(worker, bush).expect("drop");
        // The only spot inside the bounds is the bush's own, so the search runs again without bounds: the
        // first copy takes the spot to the right of the bush, and the second one the spot at (1, -1).
        board
            .set_output_placement(OutputPlacement::Free {
                footprint: (1.0, 1.0),
                bounds: Some((0.0, 0.0, 0.0, 0.0)),
            })
            .expect("a valid placement");
        board.take_events();
        let spawned: Vec<(f32, f32)> = board
            .tick(2.0)
            .into_iter()
            .filter_map(|event| match event {
                BoardEvent::CardSpawned { x, y, .. } => Some((x, y)),
                _ => None,
            })
            .collect();
        assert_eq!(spawned, vec![(1.0, 0.0), (1.0, -1.0)]);
    }

    #[test]
    fn a_search_without_bounds_looks_past_the_first_rings() {
        // The bush and eight stones fill rings 0 and 1 around (1, 0). The nearest free spots are two steps
        // out, and (1, -2) is the one with the smallest y.
        let table = [
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, -1.0),
            (2.0, -1.0),
            (0.0, -1.0),
            (2.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0),
            (2.0, 1.0),
        ];
        assert_eq!(
            free_spot(&table, (1.0, 0.0), (1.0, 1.0), None),
            Some((1.0, -2.0))
        );
        // Bounds that hold only the bush's own spot find none.
        assert_eq!(
            free_spot(&table, (1.0, 0.0), (1.0, 1.0), Some((0.0, 0.0, 0.0, 0.0))),
            None
        );
    }

    #[test]
    fn the_search_without_bounds_stops_at_the_ring_cap() {
        // The stack is so far away, in steps of this footprint, that the last ring would lie past the cap.
        let obstacles = [(1.0e3_f32, 0.0_f32)];
        let search = Search {
            obstacles: &obstacles,
            near: (0.0, 0.0),
            size: (1.0e-30, 1.0e-30),
            bounds: None,
            tolerance: 0.0,
            best: None,
        };
        assert_eq!(search.ring_range(), (0, MAX_FREE_RING));
    }

    #[test]
    fn a_footprint_too_small_for_the_stopping_rule_still_ends() {
        // The squared distances of the rings round to zero here, so the stopping rule never fires: the cap ends it.
        // The spot is the one the JavaScript search picks too (spot.test.ts), binary32 value for binary32 value.
        assert_eq!(
            free_spot(&[(1.0e3, 0.0)], (1.0e-30, 0.0), (1.0e-30, 1.0e-30), None),
            Some((-9.99e-28_f32, -1.0e-27_f32))
        );
    }
}
