//! # velvet-sfx
//!
//! Sound effects made by code, so a prototype sounds alive without a single sound file: a [`Sound`] is mono samples
//! built from tones, noise, envelopes and a gentle low-pass, and [`presets`] gives the usual ones of a desk game (a
//! stamp, paper, a chime, a buzz, coins, a click, a door). The [`Player`] plays them through the default output and
//! stays silent (never fails) when there is no sound device or `VELVET_MUTE` is set.
//!
//! ```
//! use velvet_sfx::presets;
//! let thud = presets::stamp();
//! assert!(thud.seconds() > 0.1 && thud.peak() <= 1.0);
//! ```

#[cfg(feature = "playback")]
mod player;
pub mod presets;

#[cfg(feature = "playback")]
pub use player::Player;

/// The sample rate of every sound.
pub const RATE: u32 = 44_100;

/// A mono sound: samples from -1 to 1 at [`RATE`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sound {
    pub samples: Vec<f32>,
}

/// The shape of a tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Square,
}

impl Sound {
    /// Silence of a length.
    pub fn silence(seconds: f32) -> Sound {
        Sound { samples: vec![0.0; frames(seconds)] }
    }

    /// A tone gliding from `from` to `to` hertz over its length.
    pub fn tone(wave: Wave, from: f32, to: f32, seconds: f32) -> Sound {
        let n = frames(seconds);
        let mut phase = 0.0f32;
        let samples = (0..n)
            .map(|i| {
                let t = i as f32 / n.max(1) as f32;
                phase = (phase + (from + (to - from) * t) / RATE as f32).fract();
                match wave {
                    Wave::Sine => (phase * std::f32::consts::TAU).sin(),
                    Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                    Wave::Square => if phase < 0.5 { 1.0 } else { -1.0 },
                }
            })
            .collect();
        Sound { samples }
    }

    /// White noise, the same for the same seed.
    pub fn noise(seconds: f32, seed: u32) -> Sound {
        let mut state = seed.wrapping_mul(2_654_435_761).max(1);
        let samples = (0..frames(seconds))
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as f32 / u32::MAX as f32 * 2.0 - 1.0
            })
            .collect();
        Sound { samples }
    }

    /// Shapes the loudness: up in `attack` seconds, then down exponentially, halving every `half_life` seconds.
    pub fn envelope(mut self, attack: f32, half_life: f32) -> Sound {
        let attack_frames = frames(attack).max(1) as f32;
        let k = (0.5f32).powf(1.0 / (half_life.max(1e-4) * RATE as f32));
        let mut fall = 1.0;
        for (i, s) in self.samples.iter_mut().enumerate() {
            let rise = (i as f32 / attack_frames).min(1.0);
            if i as f32 >= attack_frames {
                fall *= k;
            }
            *s *= rise * fall;
        }
        self
    }

    /// A one-pole low-pass: keeps what is under about `cutoff` hertz (soft, papery or muffled sounds).
    pub fn low_pass(mut self, cutoff: f32) -> Sound {
        let rc = 1.0 / (std::f32::consts::TAU * cutoff.max(1.0));
        let dt = 1.0 / RATE as f32;
        let a = dt / (rc + dt);
        let mut y = 0.0;
        for s in &mut self.samples {
            y += a * (*s - y);
            *s = y;
        }
        self
    }

    /// Multiplies the loudness.
    pub fn gain(mut self, g: f32) -> Sound {
        for s in &mut self.samples {
            *s *= g;
        }
        self
    }

    /// This sound with another laid over it, starting `at` seconds in.
    pub fn mix(mut self, other: &Sound, at: f32) -> Sound {
        let start = frames(at);
        if self.samples.len() < start + other.samples.len() {
            self.samples.resize(start + other.samples.len(), 0.0);
        }
        for (i, s) in other.samples.iter().enumerate() {
            self.samples[start + i] += s;
        }
        self
    }

    /// Scales the sound down when it would clip, so it never goes past 1.
    pub fn limit(mut self) -> Sound {
        let peak = self.peak();
        if peak > 0.98 {
            let g = 0.98 / peak;
            for s in &mut self.samples {
                *s *= g;
            }
        }
        self
    }

    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / RATE as f32
    }

    /// The loudest sample, as a positive value.
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }
}

fn frames(seconds: f32) -> usize {
    (seconds.max(0.0) * RATE as f32).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_have_their_length_and_stay_in_range() {
        let s = Sound::tone(Wave::Sine, 440.0, 440.0, 0.5);
        assert_eq!(s.samples.len(), 22_050);
        assert!(s.peak() <= 1.0 && s.peak() > 0.99);
    }

    #[test]
    fn envelopes_fade_and_mixes_extend() {
        let s = Sound::tone(Wave::Sine, 300.0, 300.0, 1.0).envelope(0.01, 0.1);
        let early = s.samples[441..1323].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let late = s.samples[40_000..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(early > 0.5 && late < 0.01, "{early} {late}");
        let mixed = Sound::silence(0.1).mix(&Sound::silence(0.2), 0.05);
        assert!((mixed.seconds() - 0.25).abs() < 1e-3);
    }

    #[test]
    fn noise_repeats_with_its_seed_and_low_pass_softens_it() {
        assert_eq!(Sound::noise(0.01, 7), Sound::noise(0.01, 7));
        assert_ne!(Sound::noise(0.01, 7), Sound::noise(0.01, 8));
        let loud = Sound::noise(0.2, 3);
        let soft = loud.clone().low_pass(400.0);
        assert!(soft.peak() < loud.peak());
    }

    #[test]
    fn limit_keeps_a_loud_mix_under_one() {
        let s = Sound::tone(Wave::Square, 200.0, 200.0, 0.1).gain(3.0).limit();
        assert!(s.peak() <= 0.98 + 1e-6);
    }
}
