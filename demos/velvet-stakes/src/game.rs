//! Core Balatro-like run state (tools: velvet-cards + velvet-anim + velvet-style).

use std::collections::HashMap;

use velvet_anim::{timeline_from_plan, ChannelTrack, Pose3D, Pose3DChannel, Timeline};
use velvet_cards::{shuffle_in_place, CardZones};
use velvet_math::{Ease, Vec2};
use velvet_style::{call_style_fn, JsValue, Stylesheet};

use crate::catalog::{score_played, CardStats, HandScore};
use velvet_stakes::ui::theme::{WH, WW};

pub const HAND_SIZE: usize = 8;
pub const MAX_SELECT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Collection,
    Shop,
    Options,
    BlindInfo,
    Play,
    Pause,
    Result,
}

impl Screen {
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "title" => Self::Title,
            "collection" => Self::Collection,
            "shop" => Self::Shop,
            "options" => Self::Options,
            "blind" | "blind_info" => Self::BlindInfo,
            "play" => Self::Play,
            "pause" => Self::Pause,
            "result" => Self::Result,
            _ => return None,
        })
    }

    pub fn as_id(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Collection => "collection",
            Self::Shop => "shop",
            Self::Options => "options",
            Self::BlindInfo => "blind_info",
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Result => "result",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    WinBlind,
    LoseBlind,
    RunClear,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WinBlind => "win",
            Self::LoseBlind => "lose",
            Self::RunClear => "clear",
        }
    }
}

pub struct BlindDef {
    pub name: &'static str,
    pub target: i64,
    pub hands: u32,
    pub discards: u32,
}

pub const BLINDS: &[BlindDef] = &[
    BlindDef {
        name: "Small Blind",
        target: 250,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Big Blind",
        target: 700,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Boss Blind",
        target: 1400,
        hands: 4,
        discards: 2,
    },
];

/// Visual slot for a hand card (animated via .vcss plans + pose tools).
pub struct CardVisual {
    pub id: String,
    pub rest: Vec2,
    pub pose: Pose3D,
    pub timeline: Timeline,
}

pub struct Run {
    pub zones: CardZones,
    pub selected: Vec<bool>,
    pub score: i64,
    pub hands_left: u32,
    pub discards_left: u32,
    pub target: i64,
    pub blind_name: String,
    pub ante: usize,
    pub log: Vec<String>,
    pub last: String,
    pub money: i64,
    pub visuals: Vec<CardVisual>,
    pub deal_t: f32,
}

impl Run {
    pub fn start(ante: usize, deck_cards: &[String], seed: u64, money: i64) -> Self {
        let ante = ante.min(BLINDS.len() - 1);
        let blind = &BLINDS[ante];
        let mut ids = deck_cards.to_vec();
        shuffle_in_place(&mut ids, seed);
        let mut zones = CardZones {
            library: ids,
            hand: Vec::new(),
            discard: Vec::new(),
        };
        let _ = zones.draw(HAND_SIZE.min(zones.library.len()));
        let mut run = Self {
            zones,
            selected: vec![false; HAND_SIZE],
            score: 0,
            hands_left: blind.hands,
            discards_left: blind.discards,
            target: blind.target,
            blind_name: blind.name.into(),
            ante,
            log: vec![format!("{} — target {}", blind.name, blind.target)],
            last: String::new(),
            money,
            visuals: Vec::new(),
            deal_t: 0.0,
        };
        // deal anim applied later with stylesheet
        run.rebuild_visuals(false, None);
        run
    }

    pub fn hand_slot_pos(i: usize, n: usize) -> Vec2 {
        let n = n.max(1) as f32;
        let total_w = (n - 1.0) * 91.0;
        let table_center_x = (351.0 + 988.0) * 0.5;
        let x0 = table_center_x - total_w * 0.5;
        Vec2::new(x0 + i as f32 * 91.0, 506.0)
    }

    /// Rebuild hand visuals. When `animate_deal`, prefers `.vcss` `@script dealHand` timelines.
    pub fn rebuild_visuals(&mut self, animate_deal: bool, sheet: Option<&Stylesheet>) {
        let n = self.zones.hand.len();
        self.selected.resize(n, false);
        self.visuals.clear();

        let style_run = if animate_deal {
            sheet.and_then(|s| call_style_fn(s, "dealHand", &[JsValue::num(n as f32)]).ok())
        } else {
            None
        };

        for (i, id) in self.zones.hand.iter().enumerate() {
            let rest = Self::hand_slot_pos(i, n);
            let mut pose = Pose3D::flat(rest);
            let mut timeline = Timeline::new();

            if animate_deal {
                let from = Vec2::new(WW as f32 * 0.5, WH as f32 * 0.35);
                pose.pos = from;
                pose.opacity = 0.0;
                pose.yaw = 0.8;
                pose.scale = 0.7;
                let delay = i as f32 * 0.08;
                let target = format!("card{i}");

                // Prefer channels from .vcss @script play("deal")
                if let Some(run) = &style_run {
                    if let Some(plan) = run
                        .timelines
                        .iter()
                        .find(|p| p.target.as_deref() == Some(target.as_str()))
                    {
                        timeline = timeline_from_plan(plan);
                    }
                }

                // Always move pack → slot (layout is runtime, not stylesheet)
                if timeline.channels.is_empty() {
                    timeline = Timeline::new()
                        .with_channel(
                            ChannelTrack::new(Pose3DChannel::Opacity)
                                .key(delay, 0.0, Ease::Linear)
                                .key(delay + 0.15, 1.0, Ease::QuadOut),
                        )
                        .with_channel(
                            ChannelTrack::new(Pose3DChannel::Yaw)
                                .key(delay, 0.9, Ease::Linear)
                                .key(delay + 0.28, 0.0, Ease::CubicOut),
                        )
                        .with_channel(
                            ChannelTrack::new(Pose3DChannel::Scale)
                                .key(delay, 0.65, Ease::Linear)
                                .key(delay + 0.28, 1.0, Ease::BackOut),
                        );
                }
                timeline = timeline
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::X)
                            .key(delay, from.x, Ease::Linear)
                            .key(delay + 0.28, rest.x, Ease::CubicOut),
                    )
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::Y)
                            .key(delay, from.y, Ease::Linear)
                            .key(delay + 0.28, rest.y, Ease::BackOut),
                    );
                timeline.playing = true;
                timeline.duration = timeline.duration.max(delay + 0.32);
            } else {
                pose.opacity = 1.0;
            }

            self.visuals.push(CardVisual {
                id: id.clone(),
                rest,
                pose,
                timeline,
            });
        }
        self.deal_t = 0.0;
    }

    pub fn tick_anims(&mut self, dt: f32) {
        self.deal_t += dt;
        for v in &mut self.visuals {
            if v.timeline.playing || !v.timeline.finished() {
                v.timeline.tick(dt);
                v.timeline.apply(&mut v.pose);
            }
        }
        for (i, v) in self.visuals.iter_mut().enumerate() {
            if self.selected.get(i).copied().unwrap_or(false) {
                if v.timeline.finished() || !v.timeline.playing {
                    v.pose.pos.y = v.rest.y - 20.0;
                    v.pose.scale = 1.04;
                }
            } else if v.timeline.finished() || !v.timeline.playing {
                v.pose.pos = v.rest;
                v.pose.scale = 1.0;
                v.pose.opacity = 1.0;
            }
        }
    }

    pub fn push_log(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 6 {
            let n = self.log.len() - 6;
            self.log.drain(0..n);
        }
    }

    pub fn toggle(&mut self, i: usize) {
        if i >= self.zones.hand.len() {
            return;
        }
        if self.selected[i] {
            self.selected[i] = false;
            return;
        }
        if self.selected.iter().filter(|s| **s).count() >= MAX_SELECT {
            self.push_log(format!("Max {MAX_SELECT} cards"));
            return;
        }
        self.selected[i] = true;
    }

    pub fn selected_ids(&self) -> Vec<String> {
        self.zones
            .hand
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected.get(*i).copied().unwrap_or(false))
            .map(|(_, id)| id.clone())
            .collect()
    }

    /// Zero-based index of the active blind, clamped to the shipped run.
    pub fn round_index(&self) -> usize {
        self.ante.min(BLINDS.len() - 1)
    }

    /// One-based round number suitable for the HUD.
    pub fn round_number(&self) -> usize {
        self.round_index() + 1
    }

    /// Total number of blinds in a complete run.
    pub fn round_count(&self) -> usize {
        BLINDS.len()
    }

    /// Normalized blind progress for bars and opponent health displays.
    pub fn progress_ratio(&self) -> f32 {
        if self.target <= 0 {
            return 1.0;
        }
        (self.score.max(0) as f32 / self.target as f32).clamp(0.0, 1.0)
    }

    /// Score still required to defeat the active blind.
    pub fn score_remaining(&self) -> i64 {
        self.target.saturating_sub(self.score).max(0)
    }

    /// Number of selected cards, capped by [`MAX_SELECT`] through [`Self::toggle`].
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|selected| **selected).count()
    }

    /// Current terminal state, if the blind has ended.
    pub fn pending_outcome(&self) -> Option<Outcome> {
        if self.score >= self.target {
            Some(if self.round_number() >= self.round_count() {
                Outcome::RunClear
            } else {
                Outcome::WinBlind
            })
        } else if self.hands_left == 0 {
            Some(Outcome::LoseBlind)
        } else {
            None
        }
    }

    /// Whether the primary play action can currently resolve.
    pub fn can_play(&self) -> bool {
        self.pending_outcome().is_none() && self.hands_left > 0 && self.selected_count() > 0
    }

    /// Whether the discard action can currently resolve.
    pub fn can_discard(&self) -> bool {
        self.pending_outcome().is_none() && self.discards_left > 0 && self.selected_count() > 0
    }

    /// Cash paid once when this blind is defeated.
    pub fn blind_reward(&self) -> i64 {
        4 + self.round_index() as i64 * 2
    }

    pub fn preview_score(&self, stats: &HashMap<String, CardStats>) -> HandScore {
        score_played(&self.selected_ids(), stats)
    }

    pub fn play_selected(
        &mut self,
        stats: &HashMap<String, CardStats>,
        sheet: Option<&Stylesheet>,
    ) -> Option<Outcome> {
        if let Some(outcome) = self.pending_outcome() {
            return Some(outcome);
        }
        let ids = self.selected_ids();
        if ids.is_empty() {
            self.push_log("Select 1–5 cards");
            return None;
        }
        let sc = score_played(&ids, stats);
        self.score += sc.total;
        self.hands_left -= 1;
        self.last = format!("{}  {}×{} = +{}", sc.label, sc.chips, sc.mult, sc.total);
        self.push_log(self.last.clone());

        let bonus_draw = ids
            .iter()
            .filter_map(|id| stats.get(id))
            .map(|card| card.bonus_draw)
            .sum();

        let mut idxs: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        idxs.sort_unstable();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.refill(bonus_draw);
        self.rebuild_visuals(true, sheet);

        if let Some(outcome) = self.pending_outcome() {
            if matches!(outcome, Outcome::WinBlind | Outcome::RunClear) {
                self.money += self.blind_reward();
            }
            return Some(outcome);
        }
        None
    }

    pub fn discard_selected(&mut self, sheet: Option<&Stylesheet>) {
        if self.pending_outcome().is_some() {
            return;
        }
        if self.discards_left == 0 {
            self.push_log("No discards");
            return;
        }
        let mut idxs: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            self.push_log("Select to discard");
            return;
        }
        self.discards_left -= 1;
        idxs.sort_unstable();
        let n = idxs.len();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.push_log(format!("Discarded {n}"));
        self.refill(0);
        self.rebuild_visuals(true, sheet);
    }

    fn refill(&mut self, bonus_draw: usize) {
        let need = HAND_SIZE.saturating_sub(self.zones.hand.len()) + bonus_draw;
        for _ in 0..need {
            if self.zones.library.is_empty() {
                if self.zones.discard.is_empty() {
                    break;
                }
                self.zones.library.append(&mut self.zones.discard);
                shuffle_in_place(&mut self.zones.library, 0xDEC_A_DE + self.hands_left as u64);
                self.push_log("Shuffled discard");
            }
            let _ = self.zones.draw(1);
        }
        self.selected = vec![false; self.zones.hand.len()];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::illustrated_stats;
    use std::path::Path;

    fn stats() -> HashMap<String, CardStats> {
        illustrated_stats(Path::new("."))
            .into_iter()
            .map(|card| (card.id.clone(), card))
            .collect()
    }

    fn deck(id: &str) -> Vec<String> {
        std::iter::repeat(id.to_string()).take(20).collect()
    }

    #[test]
    fn progress_and_round_api_are_hud_ready() {
        let mut run = Run::start(1, &deck("strike"), 7, 4);
        assert_eq!(run.round_number(), 2);
        assert_eq!(run.round_count(), 3);
        assert_eq!(run.score_remaining(), 700);
        assert_eq!(run.progress_ratio(), 0.0);

        run.score = 350;
        assert_eq!(run.score_remaining(), 350);
        assert!((run.progress_ratio() - 0.5).abs() < f32::EPSILON);

        run.score = 900;
        assert_eq!(run.score_remaining(), 0);
        assert_eq!(run.progress_ratio(), 1.0);
        assert_eq!(run.pending_outcome(), Some(Outcome::WinBlind));
    }

    #[test]
    fn eight_card_hand_uses_gameplay_table_slots() {
        let first = Run::hand_slot_pos(0, HAND_SIZE);
        let last = Run::hand_slot_pos(HAND_SIZE - 1, HAND_SIZE);
        assert_eq!(first, Vec2::new(351.0, 506.0));
        assert_eq!(last, Vec2::new(988.0, 506.0));
    }

    #[test]
    fn final_hand_resolves_loss_when_target_is_missed() {
        let stats = stats();
        let mut run = Run::start(0, &deck("strike"), 3, 4);
        run.target = 10_000;
        run.hands_left = 1;
        run.toggle(0);

        assert!(run.can_play());
        assert_eq!(run.play_selected(&stats, None), Some(Outcome::LoseBlind));
        assert_eq!(run.hands_left, 0);
        assert!(!run.can_play());
        assert!(!run.can_discard());
    }

    #[test]
    fn win_and_run_clear_pay_once() {
        let stats = stats();
        let mut run = Run::start(0, &deck("fireball"), 11, 4);
        run.toggle(0);
        run.toggle(1);
        run.toggle(2);

        assert_eq!(run.play_selected(&stats, None), Some(Outcome::WinBlind));
        assert_eq!(run.money, 8);
        assert_eq!(run.play_selected(&stats, None), Some(Outcome::WinBlind));
        assert_eq!(run.money, 8, "terminal input must not pay twice");

        let mut boss = Run::start(2, &deck("fireball"), 13, 10);
        boss.target = 1;
        boss.toggle(0);
        assert_eq!(boss.play_selected(&stats, None), Some(Outcome::RunClear));
        assert_eq!(boss.money, 18);
    }

    #[test]
    fn focus_draw_comes_from_card_data() {
        let stats = stats();
        let mut run = Run::start(0, &deck("focus"), 17, 4);
        run.target = 10_000;
        run.toggle(0);
        assert_eq!(run.zones.hand.len(), HAND_SIZE);

        assert_eq!(run.play_selected(&stats, None), None);
        assert_eq!(run.zones.hand.len(), HAND_SIZE + 1);
    }

    #[test]
    fn out_of_range_round_clamps_to_final_blind() {
        let run = Run::start(usize::MAX, &deck("strike"), 19, 4);
        assert_eq!(run.ante, BLINDS.len() - 1);
        assert_eq!(run.round_number(), BLINDS.len());
        assert_eq!(run.blind_name, "Boss Blind");
    }
}
