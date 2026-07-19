//! Keyframe / timeline **tools** — you build any motion (flip, open, foil, …).

use serde::{Deserialize, Serialize};
use velvet_math::Ease;

use crate::fx3d::{Pose3D, Pose3DChannel};
use crate::tween::parse_ease;

/// Build a [`Timeline`] from a unified `.vcss` animation plan (`velvet_style::TimelinePlan`).
pub fn timeline_from_plan(plan: &velvet_style::TimelinePlan) -> Timeline {
    let mut tl = Timeline::new();
    tl.duration = plan.duration;
    tl.playing = true;
    for ch in &plan.channels {
        let Some(channel) = Pose3DChannel::parse(&ch.channel) else {
            continue;
        };
        let ease = parse_ease(&ch.ease);
        let mut track = ChannelTrack::new(channel);
        for (t, v) in &ch.keys {
            track = track.key(*t, *v, ease);
        }
        tl.channels.push(track);
    }
    tl
}

/// One keyframe on a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time in seconds (or normalized if timeline uses that mode).
    pub time: f32,
    /// Value.
    pub value: f32,
    /// Ease into this key from the previous one.
    pub ease: Ease,
}

impl Keyframe {
    /// Linear key.
    pub fn at(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            ease: Ease::Linear,
        }
    }

    /// With ease.
    pub fn eased(time: f32, value: f32, ease: Ease) -> Self {
        Self { time, value, ease }
    }
}

/// A single animated channel (list of keys sorted by time).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelTrack {
    /// Which pose field.
    pub channel: Pose3DChannel,
    /// Keys (should be time-sorted).
    pub keys: Vec<Keyframe>,
}

impl ChannelTrack {
    /// Empty track.
    pub fn new(channel: Pose3DChannel) -> Self {
        Self {
            channel,
            keys: Vec::new(),
        }
    }

    /// Builder push.
    pub fn key(mut self, time: f32, value: f32, ease: Ease) -> Self {
        self.keys.push(Keyframe::eased(time, value, ease));
        self.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        self
    }

    /// Sample at time `t` (clamped to track range).
    pub fn sample(&self, t: f32) -> Option<f32> {
        if self.keys.is_empty() {
            return None;
        }
        if self.keys.len() == 1 || t <= self.keys[0].time {
            return Some(self.keys[0].value);
        }
        let last = self.keys.last().unwrap();
        if t >= last.time {
            return Some(last.value);
        }
        for w in self.keys.windows(2) {
            let a = &w[0];
            let b = &w[1];
            if t >= a.time && t <= b.time {
                let span = (b.time - a.time).max(1e-6);
                let u = ((t - a.time) / span).clamp(0.0, 1.0);
                let e = b.ease.eval(u);
                return Some(a.value + (b.value - a.value) * e);
            }
        }
        Some(last.value)
    }
}

/// Multi-channel timeline tool applied onto a [`Pose3D`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    /// Channels you composed.
    pub channels: Vec<ChannelTrack>,
    /// Elapsed playback time.
    pub elapsed: f32,
    /// Optional hard duration (0 = derive from max key time).
    pub duration: f32,
    /// Loop when finished.
    pub looping: bool,
    /// Playing.
    pub playing: bool,
}

impl Timeline {
    /// Empty timeline.
    pub fn new() -> Self {
        Self {
            playing: true,
            ..Default::default()
        }
    }

    /// Add a channel.
    pub fn with_channel(mut self, track: ChannelTrack) -> Self {
        self.channels.push(track);
        self
    }

    /// Duration = max key time or explicit.
    pub fn effective_duration(&self) -> f32 {
        if self.duration > 0.0 {
            return self.duration;
        }
        self.channels
            .iter()
            .filter_map(|c| c.keys.last().map(|k| k.time))
            .fold(0.0f32, f32::max)
    }

    /// Finished (non-looping).
    pub fn finished(&self) -> bool {
        !self.looping && self.elapsed >= self.effective_duration() && self.effective_duration() > 0.0
    }

    /// Tick clock.
    pub fn tick(&mut self, dt: f32) {
        if !self.playing {
            return;
        }
        self.elapsed += dt;
        let d = self.effective_duration();
        if d > 0.0 && self.elapsed > d {
            if self.looping {
                self.elapsed %= d;
            } else {
                self.elapsed = d;
                self.playing = false;
            }
        }
    }

    /// Apply sampled channels onto pose (base pose fields not in tracks stay).
    pub fn apply(&self, pose: &mut Pose3D) {
        for ch in &self.channels {
            if let Some(v) = ch.sample(self.elapsed) {
                pose.set_channel(ch.channel, v);
            }
        }
    }

    /// Sample into a new pose from base.
    pub fn sample_pose(&self, mut base: Pose3D) -> Pose3D {
        self.apply(&mut base);
        base
    }
}

/// Parse a compact track line for `.vanim` / tools:
/// `track id yaw 0 0 0.4 3.14 cubic_out` → times/values pairs after channel.
/// Format: `track <channel> <t0> <v0> <t1> <v1> ... [ease <name>]`
pub fn parse_track_line(parts: &[&str]) -> Result<ChannelTrack, String> {
    // parts[0] == "track"
    if parts.len() < 4 {
        return Err("track channel t0 v0 [t1 v1 …] [ease name]".into());
    }
    let channel = Pose3DChannel::parse(parts[1]).ok_or_else(|| format!("bad channel {}", parts[1]))?;
    let mut ease = Ease::CubicOut;
    let mut end = parts.len();
    if parts.len() >= 2 && parts[parts.len() - 2] == "ease" {
        ease = parse_ease(parts[parts.len() - 1]);
        end = parts.len() - 2;
    }
    let nums: Vec<f32> = parts[2..end]
        .iter()
        .map(|s| s.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "track numbers".to_string())?;
    if nums.len() < 2 || nums.len() % 2 != 0 {
        return Err("track needs time/value pairs".into());
    }
    let mut tr = ChannelTrack::new(channel);
    for pair in nums.chunks(2) {
        tr = tr.key(pair[0], pair[1], ease);
    }
    Ok(tr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_yaws_card() {
        let track = ChannelTrack::new(Pose3DChannel::Yaw)
            .key(0.0, 0.0, Ease::Linear)
            .key(0.5, std::f32::consts::PI, Ease::CubicInOut);
        let mut tl = Timeline::new().with_channel(track);
        let mut pose = Pose3D::flat(velvet_math::Vec2::ZERO);
        tl.elapsed = 0.0;
        tl.apply(&mut pose);
        assert!((pose.yaw).abs() < 1e-4);
        tl.elapsed = 0.5;
        tl.apply(&mut pose);
        assert!((pose.yaw - std::f32::consts::PI).abs() < 1e-3);
    }

    #[test]
    fn user_builds_open_motion_without_pack_recipe() {
        // Author tools only: compose opacity + yaw + y yourself.
        let mut tl = Timeline::new()
            .with_channel(
                ChannelTrack::new(Pose3DChannel::Opacity)
                    .key(0.0, 0.0, Ease::Linear)
                    .key(0.2, 1.0, Ease::QuadOut),
            )
            .with_channel(
                ChannelTrack::new(Pose3DChannel::Yaw)
                    .key(0.0, std::f32::consts::PI, Ease::Linear)
                    .key(0.4, 0.0, Ease::CubicOut),
            )
            .with_channel(
                ChannelTrack::new(Pose3DChannel::Y)
                    .key(0.0, 400.0, Ease::Linear)
                    .key(0.4, 300.0, Ease::BackOut),
            );
        for _ in 0..30 {
            tl.tick(1.0 / 60.0);
        }
        let pose = tl.sample_pose(Pose3D {
            pos: velvet_math::Vec2::new(200.0, 400.0),
            ..Default::default()
        });
        assert!(pose.opacity > 0.5);
        assert!(pose.yaw.abs() < 1.0); // flipped toward front
    }
}
