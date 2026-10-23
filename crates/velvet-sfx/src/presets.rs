//! The usual sounds of a desk game, made by code.

use crate::{Sound, Wave};

/// A rubber stamp on paper: a low thump and a short slap.
pub fn stamp() -> Sound {
    let thump = Sound::tone(Wave::Sine, 140.0, 60.0, 0.22)
        .envelope(0.002, 0.04)
        .gain(0.9);
    let slap = Sound::noise(0.08, 11)
        .low_pass(1800.0)
        .envelope(0.001, 0.012)
        .gain(0.8);
    thump.mix(&slap, 0.0).limit()
}

/// A sheet of paper slid or picked up: a soft brush of noise.
pub fn paper() -> Sound {
    let a = Sound::noise(0.18, 21)
        .low_pass(2600.0)
        .envelope(0.03, 0.05)
        .gain(0.45);
    let b = Sound::noise(0.12, 22)
        .low_pass(1200.0)
        .envelope(0.02, 0.04)
        .gain(0.3);
    a.mix(&b, 0.05).limit()
}

/// A small bell: the next visitor, a right answer.
pub fn chime() -> Sound {
    let base = Sound::tone(Wave::Sine, 1318.5, 1318.5, 0.9)
        .envelope(0.003, 0.18)
        .gain(0.5);
    let upper = Sound::tone(Wave::Sine, 2637.0, 2637.0, 0.6)
        .envelope(0.003, 0.09)
        .gain(0.2);
    let low = Sound::tone(Wave::Sine, 659.3, 659.3, 0.9)
        .envelope(0.003, 0.25)
        .gain(0.25);
    base.mix(&upper, 0.0).mix(&low, 0.0).limit()
}

/// A short low buzz: a mistake, a refusal.
pub fn buzz() -> Sound {
    Sound::tone(Wave::Square, 110.0, 98.0, 0.32)
        .low_pass(900.0)
        .envelope(0.005, 0.12)
        .gain(0.5)
        .limit()
}

/// A few coins dropped on wood.
pub fn coins() -> Sound {
    let mut s = Sound::silence(0.05);
    for (i, f) in [2900.0, 3400.0, 3100.0, 3700.0].iter().enumerate() {
        let ring = Sound::tone(Wave::Sine, *f, *f * 0.995, 0.25)
            .envelope(0.001, 0.035)
            .gain(0.35);
        s = s.mix(&ring, i as f32 * 0.055);
    }
    s.limit()
}

/// A dry click: a button, a tick.
pub fn click() -> Sound {
    Sound::noise(0.03, 31)
        .low_pass(4000.0)
        .envelope(0.0005, 0.004)
        .gain(0.6)
        .limit()
}

/// A wooden shutter or door: a knock and a creak.
pub fn door() -> Sound {
    let knock = Sound::tone(Wave::Sine, 95.0, 70.0, 0.18)
        .envelope(0.002, 0.03)
        .gain(0.8);
    let creak = Sound::tone(Wave::Triangle, 420.0, 300.0, 0.35)
        .low_pass(1500.0)
        .envelope(0.05, 0.08)
        .gain(0.15);
    knock.mix(&creak, 0.04).limit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_sounds_and_never_clips() {
        for (name, s) in [
            ("stamp", stamp()),
            ("paper", paper()),
            ("chime", chime()),
            ("buzz", buzz()),
            ("coins", coins()),
            ("click", click()),
            ("door", door()),
        ] {
            assert!(
                s.seconds() > 0.02 && s.seconds() < 1.5,
                "{name} lasts {}",
                s.seconds()
            );
            assert!(
                s.peak() > 0.05 && s.peak() <= 0.98 + 1e-6,
                "{name} peaks at {}",
                s.peak()
            );
        }
    }
}
