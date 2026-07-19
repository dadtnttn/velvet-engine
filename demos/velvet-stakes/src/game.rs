//! Core Balatro-like run state (tools: velvet-cards + velvet-anim + velvet-style).

use std::collections::HashMap;

use velvet_anim::{timeline_from_plan, ChannelTrack, Pose3D, Pose3DChannel, Timeline};
use velvet_cards::{shuffle_in_place, CardZones};
use velvet_math::{Ease, Vec2};
use velvet_style::{call_style_fn, JsValue, Stylesheet};

use crate::catalog::{score_played, CardStats, HandScore};
use velvet_stakes::ui::theme::{WW, WH};

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
        let blind = &BLINDS[ante.min(BLINDS.len() - 1)];
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
        let total_w = (n - 1.0) * 108.0;
        let x0 = (WW as f32 - total_w) * 0.5 - 40.0;
        Vec2::new(x0 + i as f32 * 108.0, 200.0)
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
                    v.pose.pos.y = v.rest.y - 18.0;
                    v.pose.scale = 1.06;
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

    pub fn preview_score(&self, stats: &HashMap<String, CardStats>) -> HandScore {
        score_played(&self.selected_ids(), stats)
    }

    pub fn play_selected(
        &mut self,
        stats: &HashMap<String, CardStats>,
        sheet: Option<&Stylesheet>,
    ) -> Option<Outcome> {
        let ids = self.selected_ids();
        if ids.is_empty() {
            self.push_log("Select 1–5 cards");
            return None;
        }
        if self.hands_left == 0 {
            return None;
        }
        let sc = score_played(&ids, stats);
        self.score += sc.total;
        self.hands_left -= 1;
        self.last = format!("{}  {}×{} = +{}", sc.label, sc.chips, sc.mult, sc.total);
        self.push_log(self.last.clone());

        let focus_n = ids.iter().filter(|id| id.as_str() == "focus").count();

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
        self.refill(focus_n);
        self.rebuild_visuals(true, sheet);

        if self.score >= self.target {
            self.money += 4 + self.ante as i64 * 2;
            return Some(if self.ante + 1 >= BLINDS.len() {
                Outcome::RunClear
            } else {
                Outcome::WinBlind
            });
        }
        if self.hands_left == 0 {
            return Some(Outcome::LoseBlind);
        }
        None
    }

    pub fn discard_selected(&mut self, sheet: Option<&Stylesheet>) {
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
                shuffle_in_place(
                    &mut self.zones.library,
                    0xDEC_A_DE + self.hands_left as u64,
                );
                self.push_log("Shuffled discard");
            }
            let _ = self.zones.draw(1);
        }
        self.selected = vec![false; self.zones.hand.len()];
    }
}
