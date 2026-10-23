//! Labels that belong to things in the world (a regiment, an army, a person): each comes with where
//! its thing shows on the screen (the top of what is seen of it, and a point on it), and the layout
//! puts every label just over its thing, keeps it off the HUD and off the other labels (the ones that
//! matter more choose first: the selected, the hovered, the near), hangs it from a pole down to its
//! thing, and lets it glide to its place and fade in and out instead of jumping. A label with no room
//! at all fades away. Screen units are the caller's (Demon Queen's war labels use its 1920×1080
//! overlay).
use std::collections::HashMap;

/// One thing to label this frame.
#[derive(Clone, Copy, Debug)]
pub struct TagAnchor {
    /// The caller's id for the thing; a label keeps its place and fade by it.
    pub id: usize,
    /// The top-centre of what is seen of the thing (the label sits over it).
    pub head: [f32; 2],
    /// A point on the thing (the pole ends there).
    pub foot: [f32; 2],
    /// The label's full size.
    pub size: [f32; 2],
    /// The compact size it may shrink to when crowded (or far).
    pub compact: [f32; 2],
    /// Higher chooses its place first (selected, hovered, near the eye).
    pub priority: f32,
    /// Start compact (far away): it never grows back while crowded.
    pub far: bool,
}

/// Where a label ended up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedTag {
    /// The anchor's id.
    pub id: usize,
    /// The label: x, y, width, height.
    pub rect: [f32; 4],
    /// The pole: from the label's foot down to the thing.
    pub pole: [[f32; 2]; 2],
    /// How visible it is (it fades in and out instead of popping).
    pub alpha: f32,
    /// Shown at its compact size.
    pub compact: bool,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    at: [f32; 2],
    alpha: f32,
    seen: f32,
}

/// The labels' memory from one frame to the next (where each was, how visible).
#[derive(Debug, Default)]
pub struct TagLayout {
    slots: HashMap<usize, Slot>,
    last: Option<f32>,
    placed: Vec<PlacedTag>,
}

const GAP: f32 = 10.0;
/// How far up a crowded label may climb (steps of its own height).
const CLIMB: usize = 4;

fn overlaps(a: [f32; 4], b: [f32; 4], pad: f32) -> bool {
    a[0] < b[0] + b[2] + pad
        && b[0] < a[0] + a[2] + pad
        && a[1] < b[1] + b[3] + pad
        && b[1] < a[1] + a[3] + pad
}

impl TagLayout {
    /// The labels as last placed (for clicks).
    pub fn placed(&self) -> &[PlacedTag] {
        &self.placed
    }

    /// Places this frame's labels at time `now` (seconds), off `avoid` and inside `bounds`.
    pub fn update(
        &mut self,
        anchors: &[TagAnchor],
        avoid: &[[f32; 4]],
        bounds: [f32; 4],
        now: f32,
    ) -> &[PlacedTag] {
        let dt = self.last.map_or(1.0, |t| (now - t).clamp(0.0, 0.25));
        self.last = Some(now);
        let mut order: Vec<&TagAnchor> = anchors.iter().collect();
        order.sort_by(|a, b| b.priority.total_cmp(&a.priority).then(a.id.cmp(&b.id)));
        let mut taken: Vec<[f32; 4]> = Vec::with_capacity(order.len());
        let mut out = Vec::with_capacity(order.len());
        let inside = |r: [f32; 4]| {
            r[0] >= bounds[0]
                && r[1] >= bounds[1]
                && r[0] + r[2] <= bounds[0] + bounds[2]
                && r[1] + r[3] <= bounds[1] + bounds[3]
        };
        for a in order {
            let free = |r: [f32; 4], taken: &Vec<[f32; 4]>| {
                inside(r)
                    && !avoid.iter().any(|h| overlaps(r, *h, 2.0))
                    && !taken.iter().any(|t| overlaps(r, *t, 3.0))
            };
            let sizes: &[([f32; 2], bool)] = if a.far {
                &[(a.compact, true)]
            } else {
                &[(a.size, false), (a.compact, true)]
            };
            let mut found = None;
            'search: for &(size, compact) in sizes {
                for step in 0..=CLIMB {
                    let lift = step as f32 * (size[1] + 4.0);
                    for dx in [0.0, -0.55, 0.55] {
                        let x = a.head[0] - size[0] / 2.0 + dx * size[0];
                        let y = a.head[1] - GAP - size[1] - lift;
                        let r = [x, y, size[0], size[1]];
                        if free(r, &taken) {
                            found = Some((r, compact));
                            break 'search;
                        }
                    }
                }
            }
            let (target, compact, visible) = match found {
                Some((r, c)) => (r, c, true),
                // no room: it fades out where it would have stood
                None => {
                    let s = if a.far { a.compact } else { a.size };
                    (
                        [a.head[0] - s[0] / 2.0, a.head[1] - GAP - s[1], s[0], s[1]],
                        a.far,
                        false,
                    )
                }
            };
            if visible {
                taken.push(target);
            }
            // glide: a label new on screen (or one that jumped far) takes its place at once
            let k = 1.0 - (-dt * 14.0).exp();
            let slot = self.slots.entry(a.id).or_insert(Slot {
                at: [target[0], target[1]],
                alpha: 0.0,
                seen: now,
            });
            let far_jump = (slot.at[0] - target[0]).hypot(slot.at[1] - target[1]) > 260.0;
            if now - slot.seen > 0.3 || far_jump {
                slot.at = [target[0], target[1]];
            } else {
                slot.at[0] += (target[0] - slot.at[0]) * k;
                slot.at[1] += (target[1] - slot.at[1]) * k;
            }
            // in fast, out faster (a label caught under the HUD must not linger there)
            let fade = 1.0 - (-dt * if visible { 10.0 } else { 22.0 }).exp();
            slot.alpha += (if visible { 1.0 } else { 0.0 } - slot.alpha) * fade;
            slot.seen = now;
            if slot.alpha < 0.02 {
                continue;
            }
            let rect = [slot.at[0], slot.at[1], target[2], target[3]];
            let base = [rect[0] + rect[2] / 2.0, rect[1] + rect[3]];
            out.push(PlacedTag {
                id: a.id,
                rect,
                pole: [base, a.foot],
                alpha: slot.alpha.clamp(0.0, 1.0),
                compact,
            });
        }
        // forget what has been gone a while
        self.slots.retain(|_, s| now - s.seen < 2.0);
        // draw the far ones first, so the chosen ones end on top
        out.reverse();
        self.placed = out;
        &self.placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(id: usize, x: f32, y: f32, priority: f32) -> TagAnchor {
        TagAnchor {
            id,
            head: [x, y],
            foot: [x, y + 40.0],
            size: [160.0, 34.0],
            compact: [44.0, 30.0],
            priority,
            far: false,
        }
    }

    #[test]
    fn crowded_labels_stack_instead_of_overlapping_and_keep_off_the_hud() {
        let mut l = TagLayout::default();
        let anchors = [
            anchor(1, 500.0, 500.0, 1.0),
            anchor(2, 510.0, 505.0, 2.0),
            anchor(3, 505.0, 498.0, 0.5),
        ];
        let hud = [[0.0, 0.0, 1920.0, 100.0]];
        let placed = l
            .update(&anchors, &hud, [0.0, 0.0, 1920.0, 1080.0], 0.0)
            .to_vec();
        assert_eq!(placed.len(), 3);
        for (i, a) in placed.iter().enumerate() {
            assert!(a.rect[1] >= 100.0, "off the HUD");
            for b in &placed[i + 1..] {
                assert!(!overlaps(a.rect, b.rect, 0.0), "{a:?} {b:?}");
            }
            // the pole hangs from the label down to its thing
            assert!(a.pole[0][1] <= a.pole[1][1]);
        }
        // the most important one sits nearest its thing
        let top = placed.iter().find(|p| p.id == 2).unwrap();
        assert!(top.rect[1] + top.rect[3] >= 505.0 - GAP - 1.0);
    }

    #[test]
    fn a_label_glides_to_a_new_place_and_one_with_no_room_fades() {
        let mut l = TagLayout::default();
        let b = [0.0, 0.0, 1920.0, 1080.0];
        l.update(&[anchor(1, 500.0, 500.0, 1.0)], &[], b, 0.0);
        let before = l.placed()[0].rect;
        l.update(&[anchor(1, 560.0, 500.0, 1.0)], &[], b, 1.0 / 60.0);
        let mid = l.placed()[0].rect;
        assert!(
            mid[0] > before[0] && mid[0] < before[0] + 60.0,
            "it moves part of the way"
        );
        // under a HUD that covers everything: it fades out over a few frames
        let hud = [[0.0, 0.0, 1920.0, 1080.0]];
        let mut t = 1.0 / 60.0;
        for _ in 0..40 {
            t += 1.0 / 60.0;
            l.update(&[anchor(1, 560.0, 500.0, 1.0)], &hud, b, t);
        }
        assert!(l.placed().is_empty());
    }
}
