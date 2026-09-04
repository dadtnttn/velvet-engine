//! Skeletal-animation blending: clips, crossfades, layers, bone masks.
//!
//! The renderer samples authored glTF clips to vertices; this module is
//! the layer above it for *combining* motion: name-addressed tracks
//! (stable across files, unlike node indices), normalized-lerp
//! quaternion blending (same convention as the renderer sampler), masked
//! layers (e.g. legs vs torso), and timed crossfades. Everything is pure:
//! [`AnimMixer3D::tick`] advances clocks, [`AnimMixer3D::pose`] builds
//! the blended node sample — deterministic for replays and tests.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use velvet_math::Vec3;

/// Which node channel a track drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel3D {
    /// Local translation.
    Translation,
    /// Local rotation quaternion `[x, y, z, w]`.
    Rotation,
    /// Local scale.
    Scale,
}

/// One keyframed track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimTrack3D {
    /// Key times in seconds (ascending).
    pub times: Vec<f32>,
    /// Node-local translations (matches `times` when used).
    #[serde(default)]
    pub translations: Vec<Vec3>,
    /// Node-local rotation quaternions `[x, y, z, w]`.
    #[serde(default)]
    pub rotations: Vec<[f32; 4]>,
    /// Node-local scales.
    #[serde(default)]
    pub scales: Vec<Vec3>,
}

/// One named animation clip with name-addressed tracks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimClip3D {
    /// Stable clip name.
    pub name: String,
    /// Duration in seconds (0 = static pose).
    pub duration: f32,
    /// Tracks as `(node_name, channel, track)`.
    #[serde(default)]
    pub tracks: Vec<(String, Channel3D, AnimTrack3D)>,
}

/// One blended node sample.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSample3D {
    /// Node name.
    pub node: String,
    /// Blended translation (None = untouched by every layer).
    pub translation: Option<Vec3>,
    /// Blended rotation (None = untouched).
    pub rotation: Option<[f32; 4]>,
    /// Blended scale (None = untouched).
    pub scale: Option<Vec3>,
}

/// Identity quaternion.
pub const IDENTITY_QUAT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Normalize a quaternion (zero-safe).
pub fn normalize_quat(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        .sqrt()
        .max(1e-6);
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

/// Normalized lerp between quaternions (short arc, renderer convention).
pub fn quat_nlerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut b = b;
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
    }
    normalize_quat([
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ])
}

fn sample_track_vec3(times: &[f32], values: &[Vec3], time: f32) -> Option<Vec3> {
    if times.is_empty() || values.is_empty() || times.len() != values.len() {
        return None;
    }
    if values.len() == 1 || time <= times[0] {
        return Some(values[0]);
    }
    for (i, window) in times.windows(2).enumerate() {
        if time <= window[1] {
            let t = ((time - window[0]) / (window[1] - window[0]).max(1e-6)).clamp(0.0, 1.0);
            return Some(values[i].lerp(values[i + 1], t));
        }
    }
    values.last().copied()
}

/// Sample one track at `time` (linear between keys, no looping here).
pub fn sample_track_value(
    track: &AnimTrack3D,
    channel: Channel3D,
    time: f32,
) -> Option<TrackSample> {
    match channel {
        Channel3D::Translation => {
            sample_track_vec3(&track.times, &track.translations, time).map(TrackSample::Vec3)
        }
        Channel3D::Scale => {
            sample_track_vec3(&track.times, &track.scales, time).map(TrackSample::Vec3)
        }
        Channel3D::Rotation => {
            let q = sample_track_quat(&track.times, &track.rotations, time)?;
            Some(TrackSample::Quat(q))
        }
    }
}

/// Sampled track value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackSample {
    /// Vec3 channel.
    Vec3(Vec3),
    /// Quaternion channel.
    Quat([f32; 4]),
}

fn sample_track_quat(times: &[f32], values: &[[f32; 4]], time: f32) -> Option<[f32; 4]> {
    if times.is_empty() || values.is_empty() || times.len() != values.len() {
        return None;
    }
    if values.len() == 1 || time <= times[0] {
        return Some(normalize_quat(values[0]));
    }
    for (i, window) in times.windows(2).enumerate() {
        let (a_time, b_time) = (window[0], window[1]);
        if time <= b_time {
            let t = ((time - a_time) / (b_time - a_time).max(1e-6)).clamp(0.0, 1.0);
            return Some(quat_nlerp(values[i], values[i + 1], t));
        }
    }
    values.last().copied().map(normalize_quat)
}

/// Sample a whole clip at `time` (loops when duration > 0).
pub fn sample_clip(clip: &AnimClip3D, time: f32) -> Vec<(String, Channel3D, TrackSample)> {
    let t = if clip.duration > 0.0 {
        time.rem_euclid(clip.duration)
    } else {
        0.0
    };
    let mut out = Vec::new();
    for (node, channel, track) in &clip.tracks {
        if let Some(sample) = sample_track_value(track, *channel, t) {
            out.push((node.clone(), *channel, sample));
        }
    }
    out
}

/// One mixer layer: a clip with weight, mask, and its own clock.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimLayer3D {
    /// Clip index into the mixer bank.
    pub clip: usize,
    /// Layer weight 0..=1.
    pub weight: f32,
    /// Node-name prefixes this layer may touch (empty = all nodes).
    pub mask: Vec<String>,
    /// Clip clock in seconds.
    pub time: f32,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Loop the clip.
    pub looping: bool,
}

/// Crossfade state between two layers.
#[derive(Debug, Clone, PartialEq)]
struct Crossfade3D {
    /// Source layer snapshot (clip, time, weight baseline 1).
    from_clip: usize,
    /// Source clock (frozen during the fade).
    from_time: f32,
    /// Fade duration seconds.
    duration: f32,
    /// Elapsed fade time.
    elapsed: f32,
}

/// Layered animation mixer with crossfades and bone masks.
#[derive(Debug, Clone)]
pub struct AnimMixer3D {
    /// Clip bank.
    pub clips: Vec<AnimClip3D>,
    /// Active layers (index 0 = base).
    pub layers: Vec<AnimLayer3D>,
    /// Active crossfade, if any.
    crossfade: Option<Crossfade3D>,
}

impl AnimMixer3D {
    /// Empty mixer.
    pub fn new() -> Self {
        Self {
            clips: Vec::new(),
            layers: Vec::new(),
            crossfade: None,
        }
    }

    /// Clip index by name.
    pub fn clip_index(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    /// Play a clip on the base layer with a crossfade from the current pose.
    ///
    /// `fade == 0` cuts instantly. Missing names are ignored (keeps the
    /// mixer total — authors validate clip names at load).
    pub fn play(&mut self, name: &str, fade: f32) {
        let Some(index) = self.clip_index(name) else {
            return;
        };
        if fade <= 0.0 || self.layers.is_empty() {
            self.layers = vec![AnimLayer3D {
                clip: index,
                weight: 1.0,
                mask: Vec::new(),
                time: 0.0,
                speed: 1.0,
                looping: true,
            }];
            self.crossfade = None;
            return;
        }
        let from = self.layers[0].clone();
        self.layers[0] = AnimLayer3D {
            clip: index,
            weight: 1.0,
            mask: Vec::new(),
            time: 0.0,
            speed: 1.0,
            looping: true,
        };
        self.crossfade = Some(Crossfade3D {
            from_clip: from.clip,
            from_time: from.time,
            duration: fade.max(1e-4),
            elapsed: 0.0,
        });
    }

    /// Push an additive-feel overlay layer (e.g. wounded torso over run).
    ///
    /// Returns the layer index for [`Self::set_layer_weight`]. Missing
    /// names still push nothing and return `None`.
    pub fn push_layer(
        &mut self,
        name: &str,
        weight: f32,
        mask: Vec<String>,
        looping: bool,
    ) -> Option<usize> {
        let index = self.clip_index(name)?;
        self.layers.push(AnimLayer3D {
            clip: index,
            weight: weight.clamp(0.0, 1.0),
            mask,
            time: 0.0,
            speed: 1.0,
            looping,
        });
        Some(self.layers.len() - 1)
    }

    /// Set an overlay layer weight.
    pub fn set_layer_weight(&mut self, layer: usize, weight: f32) {
        if let Some(l) = self.layers.get_mut(layer) {
            l.weight = weight.clamp(0.0, 1.0);
        }
    }

    /// Advance all clocks (and any crossfade) by `dt`.
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for layer in &mut self.layers {
            layer.time += dt * layer.speed;
        }
        if let Some(fade) = &mut self.crossfade {
            fade.elapsed += dt;
        }
        if self
            .crossfade
            .as_ref()
            .is_some_and(|f| f.elapsed >= f.duration)
        {
            self.crossfade = None;
        }
    }

    /// Blended pose across base, crossfade source, and overlay layers.
    pub fn pose(&self) -> Vec<NodeSample3D> {
        let mut acc: BTreeMap<String, NodeSample3D> = BTreeMap::new();
        // Crossfade source first (weight ramps 1 -> 0).
        if let Some(fade) = &self.crossfade {
            let k = 1.0 - (fade.elapsed / fade.duration).clamp(0.0, 1.0);
            if k > 0.0 {
                if let Some(clip) = self.clips.get(fade.from_clip) {
                    apply_samples(&mut acc, sample_clip(clip, fade.from_time), k, &[]);
                }
            }
        }
        for (i, layer) in self.layers.iter().enumerate() {
            let Some(clip) = self.clips.get(layer.clip) else {
                continue;
            };
            // Base layer during a fade ramps 0 -> 1; overlays use weight.
            let mut weight = layer.weight;
            if i == 0 {
                if let Some(fade) = &self.crossfade {
                    weight *= (fade.elapsed / fade.duration).clamp(0.0, 1.0);
                }
            }
            if weight <= 0.0 {
                continue;
            }
            // Non-looping layers clamp at the ends instead of rem_euclid.
            let samples = if layer.looping {
                sample_clip(clip, layer.time)
            } else {
                sample_clip_noloop(clip, layer.time)
            };
            apply_samples(&mut acc, samples, weight, &layer.mask);
        }
        acc.into_values().collect()
    }
}

impl Default for AnimMixer3D {
    fn default() -> Self {
        Self::new()
    }
}

/// Sample without looping (clamps at the ends).
fn sample_clip_noloop(clip: &AnimClip3D, time: f32) -> Vec<(String, Channel3D, TrackSample)> {
    let t = time.clamp(0.0, clip.duration.max(0.0));
    let mut out = Vec::new();
    for (node, channel, track) in &clip.tracks {
        if let Some(sample) = sample_track_value(track, *channel, t) {
            out.push((node.clone(), *channel, sample));
        }
    }
    out
}

fn layer_allows(mask: &[String], node: &str) -> bool {
    mask.is_empty() || mask.iter().any(|prefix| node.starts_with(prefix.as_str()))
}

fn apply_samples(
    acc: &mut BTreeMap<String, NodeSample3D>,
    samples: Vec<(String, Channel3D, TrackSample)>,
    weight: f32,
    mask: &[String],
) {
    for (node, channel, sample) in samples {
        if !layer_allows(mask, &node) {
            continue;
        }
        let entry = acc.entry(node.clone()).or_insert(NodeSample3D {
            node,
            translation: None,
            rotation: None,
            scale: None,
        });
        match (channel, sample) {
            (Channel3D::Translation, TrackSample::Vec3(v)) => {
                entry.translation = Some(match entry.translation {
                    Some(cur) => cur.lerp(v, weight),
                    None if weight >= 1.0 => v,
                    None => Vec3::ZERO.lerp(v, weight),
                });
            }
            (Channel3D::Scale, TrackSample::Vec3(v)) => {
                entry.scale = Some(match entry.scale {
                    Some(cur) => cur.lerp(v, weight),
                    None if weight >= 1.0 => v,
                    None => Vec3::new(1.0, 1.0, 1.0).lerp(v, weight),
                });
            }
            (Channel3D::Rotation, TrackSample::Quat(q)) => {
                entry.rotation = Some(match entry.rotation {
                    Some(cur) => quat_nlerp(cur, q, weight),
                    None if weight >= 1.0 => q,
                    None => quat_nlerp(IDENTITY_QUAT, q, weight),
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track2(t1: f32, v1: Vec3, t2: f32, v2: Vec3) -> AnimTrack3D {
        AnimTrack3D {
            times: vec![t1, t2],
            translations: vec![v1, v2],
            rotations: Vec::new(),
            scales: Vec::new(),
        }
    }

    fn walk_clip() -> AnimClip3D {
        AnimClip3D {
            name: "walk".into(),
            duration: 1.0,
            tracks: vec![(
                "Root".into(),
                Channel3D::Translation,
                track2(0.0, Vec3::ZERO, 1.0, Vec3::new(0.0, 0.0, 2.0)),
            )],
        }
    }

    #[test]
    fn sample_loops_clip_time() {
        let clip = walk_clip();
        let at = |t: f32| match sample_clip(&clip, t)[0].2 {
            TrackSample::Vec3(v) => v,
            _ => Vec3::ZERO,
        };
        assert_eq!(at(0.0), Vec3::ZERO);
        assert_eq!(at(0.5), Vec3::new(0.0, 0.0, 1.0));
        // Loops: t=1.5 behaves like t=0.5.
        assert_eq!(at(1.5), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn crossfade_blends_old_into_new() {
        let idle = AnimClip3D {
            name: "idle".into(),
            duration: 1.0,
            tracks: vec![(
                "Root".into(),
                Channel3D::Translation,
                track2(0.0, Vec3::ZERO, 1.0, Vec3::ZERO),
            )],
        };
        let mut mixer = AnimMixer3D::new();
        mixer.clips = vec![idle, walk_clip()];
        mixer.play("idle", 0.0);
        mixer.tick(0.5);
        mixer.play("walk", 0.5);
        // Mid-fade: half idle-at-0.5 (0) + half walk-at-0.25 (z=0.5).
        mixer.tick(0.25);
        let pose = mixer.pose();
        assert_eq!(pose.len(), 1);
        assert_eq!(pose[0].translation, Some(Vec3::new(0.0, 0.0, 0.25)));
        // Fade done: pure walk clock (0.25s in).
        mixer.tick(0.5);
        let pose = mixer.pose();
        assert_eq!(pose[0].translation, Some(Vec3::new(0.0, 0.0, 1.5)));
    }

    #[test]
    fn mask_restricts_overlay_nodes() {
        let wave = AnimClip3D {
            name: "wave".into(),
            duration: 1.0,
            tracks: vec![
                (
                    "ArmL".into(),
                    Channel3D::Translation,
                    track2(0.0, Vec3::ZERO, 1.0, Vec3::new(0.0, 2.0, 0.0)),
                ),
                (
                    "LegR".into(),
                    Channel3D::Translation,
                    track2(0.0, Vec3::ZERO, 1.0, Vec3::new(9.0, 9.0, 9.0)),
                ),
            ],
        };
        let mut mixer = AnimMixer3D::new();
        mixer.clips = vec![walk_clip(), wave];
        mixer.play("walk", 0.0);
        mixer.push_layer("wave", 1.0, vec!["Arm".into()], true);
        mixer.tick(0.5);
        let pose = mixer.pose();
        let arm = pose.iter().find(|n| n.node == "ArmL").unwrap();
        assert_eq!(arm.translation, Some(Vec3::new(0.0, 1.0, 0.0)));
        // LegR is masked out: only layers touching it contribute (none).
        assert!(pose.iter().find(|n| n.node == "LegR").is_none());
        // Base still drives Root.
        assert!(pose.iter().any(|n| n.node == "Root"));
    }

    #[test]
    fn quat_blend_takes_short_arc() {
        let q = quat_nlerp(IDENTITY_QUAT, [0.0, 0.0, 0.0, -1.0], 0.5);
        assert!((q[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn unknown_clip_play_is_ignored() {
        let mut mixer = AnimMixer3D::new();
        mixer.play("nope", 0.0);
        assert!(mixer.layers.is_empty());
        assert!(mixer.pose().is_empty());
    }
}
