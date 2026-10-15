//! The living background of each place, all synthesised at start-up like the
//! rest of the game's sound: wind and leaves in the forest, a village's
//! murmur and water, the castle's torches and stone; birds, creaking
//! branches, a hammer, a far bell, a drip, and steps that match the ground.
//! Continuous beds fade with the zone (and crickets with the night); the
//! one-shots are scheduled at random and placed around the listener.
use super::*;

/// One-shot ambient sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Amb {
    // Forest.
    BirdTweet,
    BirdTrill,
    BirdSong,
    Dove,
    Crow,
    Woodpecker,
    Owl,
    BranchCreak,
    TwigSnap,
    LeafGust,
    // Village.
    Hammer,
    BellFar,
    DoorCreak,
    Splash,
    Chime,
    // Castle.
    TorchWhoosh,
    StoneCreak,
    Chain,
    Cloth,
    WindHowl,
    FarDoor,
    CandlePop,
    // The brothel.
    FireCrackle,
    GlassClink,
    Pour,
    Quill,
    Giggle,
    Moan,
    MoanRise,
    Sigh,
    BedCreak,
    // The hot springs.
    Bubble,
    Slosh,
    Clack,
}

impl Amb {
    pub(crate) const ALL: [Amb; 34] = [
        Amb::BirdTweet,
        Amb::BirdTrill,
        Amb::BirdSong,
        Amb::Dove,
        Amb::Crow,
        Amb::Woodpecker,
        Amb::Owl,
        Amb::BranchCreak,
        Amb::TwigSnap,
        Amb::LeafGust,
        Amb::Hammer,
        Amb::BellFar,
        Amb::DoorCreak,
        Amb::Splash,
        Amb::Chime,
        Amb::TorchWhoosh,
        Amb::StoneCreak,
        Amb::Chain,
        Amb::Cloth,
        Amb::WindHowl,
        Amb::FarDoor,
        Amb::CandlePop,
        Amb::FireCrackle,
        Amb::GlassClink,
        Amb::Pour,
        Amb::Quill,
        Amb::Giggle,
        Amb::Moan,
        Amb::MoanRise,
        Amb::Sigh,
        Amb::BedCreak,
        Amb::Bubble,
        Amb::Slosh,
        Amb::Clack,
    ];
}

// ---------------------------------------------------------------- synthesis

/// A bird's note: a pitch glide with a little vibrato and a rounded swell.
#[allow(clippy::too_many_arguments)]
fn note(
    out: &mut [f32],
    at: f32,
    f0: f32,
    f1: f32,
    length: f32,
    gain: f32,
    vibrato: (f32, f32),
    overtone: f32,
) {
    let start = (at * RATE as f32) as usize;
    let n = (length * RATE as f32) as usize;
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = t_of(i);
        let u = i as f32 / n.max(1) as f32;
        let f = f0 + (f1 - f0) * u + vibrato.0 * (std::f32::consts::TAU * vibrato.1 * t).sin();
        phase += std::f32::consts::TAU * f / RATE as f32;
        let swell = (u * std::f32::consts::PI).sin().powf(0.7);
        if let Some(s) = out.get_mut(start + i) {
            *s += (phase.sin() + (phase * 2.0).sin() * overtone) * swell * gain;
        }
    }
}

/// Interleave two mono channels.
fn stereo(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter().zip(right).flat_map(|(l, r)| [*l, *r]).collect()
}

/// A gust envelope of whole cycles per loop, so the loop closes.
fn gust(t: f32, seconds: f32, phase: f32) -> f32 {
    let a = (std::f32::consts::TAU * t / seconds * 2.0 + phase).sin() * 0.5 + 0.5;
    let b = (std::f32::consts::TAU * t / seconds * 3.0 + phase * 1.7).sin() * 0.5 + 0.5;
    0.30 + 0.70 * (a * 0.65 + b * 0.35).powf(1.6)
}

fn forest_channel(seed: u32, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(seed);
    // Wind in the crowns: a low breath that wanders in pitch.
    let mut wind = brown(&mut noise, n);
    lowpass(&mut wind, 650.0);
    swept_bandpass(
        &mut wind,
        |u| 320.0 + 260.0 * ((u * std::f32::consts::TAU * 2.0).sin() * 0.5 + 0.5),
        0.9,
    );
    // Leaves: bright noise that swells with the gusts.
    let mut leaves: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    highpass(&mut leaves, 2200.0);
    lowpass(&mut leaves, 7200.0);
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let t = t_of(i);
        let g = gust(t, seconds, seed as f32 * 0.001);
        let shimmer = 0.75 + 0.25 * (t * std::f32::consts::TAU * 9.0 + seed as f32).sin();
        out[i] = wind[i] * 2.4 * g + leaves[i] * 0.20 * g.powf(1.5) * shimmer;
    }
    // Now and then a dry rustle.
    for _ in 0..(seconds * 2.0) as usize {
        let at = noise.next().abs() * (seconds - 0.2);
        click(
            &mut out,
            at,
            2600.0 + noise.next().abs() * 2600.0,
            1.4,
            0.05 + noise.next().abs() * 0.06,
            0.05,
            &mut noise,
        );
    }
    out
}

/// Wind, leaves and the odd rustle (a seamless stereo loop).
fn forest_bed() -> Vec<f32> {
    let seconds = 20.0;
    let (l, r) = (
        forest_channel(0xF04E_57A1, seconds),
        forest_channel(0x0BEE_5CE1, seconds),
    );
    let peak = l
        .iter()
        .chain(&r)
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.55 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        l.iter().map(|s| s * scale).collect(),
        r.iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

fn village_channel(seed: u32, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(seed);
    let mut wind = brown(&mut noise, n);
    lowpass(&mut wind, 450.0);
    // A crowd far off: voiced noise in the speech band, chopped at the pace
    // of talk.
    let mut murmur: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    bandpass(&mut murmur, 620.0, 0.9);
    let mut murmur_hi: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    bandpass(&mut murmur_hi, 1500.0, 1.1);
    // Water in a fountain: fine, fast sparkle.
    let mut water: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    highpass(&mut water, 1800.0);
    lowpass(&mut water, 6500.0);
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let t = t_of(i);
        let syll = (t * std::f32::consts::TAU * 3.0).sin() * 0.5 + 0.5;
        let syll2 = (t * std::f32::consts::TAU * 4.6 + seed as f32).sin() * 0.5 + 0.5;
        let talk = 0.25 + 0.75 * (syll * syll2).powf(0.8);
        let swell =
            0.5 + 0.5 * (t * std::f32::consts::TAU / seconds * 2.0 + seed as f32 * 0.3).sin();
        let ripple = 0.6 + 0.4 * (t * std::f32::consts::TAU * 11.0).sin();
        out[i] = wind[i] * 1.6 * gust(t, seconds, seed as f32 * 0.0007)
            + (murmur[i] * 1.5 + murmur_hi[i] * 0.9) * 0.16 * talk * (0.5 + 0.5 * swell)
            + water[i] * 0.05 * ripple;
    }
    out
}

/// Distant voices, a light wind and a fountain (a seamless stereo loop).
fn village_bed() -> Vec<f32> {
    let seconds = 20.0;
    let (l, r) = (
        village_channel(0x71_1A9E, seconds),
        village_channel(0x0F00_71A6, seconds),
    );
    let peak = l
        .iter()
        .chain(&r)
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.5 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        l.iter().map(|s| s * scale).collect(),
        r.iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

fn castle_channel(seed: u32, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(seed);
    // The hush of a stone hall.
    let mut room = brown(&mut noise, n);
    lowpass(&mut room, 210.0);
    lowpass(&mut room, 210.0);
    // Wind finding a way in through a slit.
    let mut whistle: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    swept_bandpass(
        &mut whistle,
        |u| 380.0 + 140.0 * (u * std::f32::consts::TAU * 3.0).sin(),
        9.0,
    );
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let t = t_of(i);
        let draught = 0.5 + 0.5 * (t * std::f32::consts::TAU / seconds * 2.0 + seed as f32).sin();
        let hum = (t * std::f32::consts::TAU * 55.0).sin() * 0.05
            + (t * std::f32::consts::TAU * 82.5).sin() * 0.025;
        out[i] = room[i] * 2.6 + hum + whistle[i] * 0.55 * draught.powi(2);
    }
    // Torches and the fire: a steady crackle with the odd pop.
    let mut fire = vec![0.0f32; n];
    for _ in 0..(seconds * 30.0) as usize {
        let at = noise.next().abs() * (seconds - 0.1);
        click(
            &mut fire,
            at,
            900.0 + noise.next().abs() * 2400.0,
            2.0,
            0.012 + noise.next().abs() * 0.02,
            0.05 + noise.next().abs() * 0.08,
            &mut noise,
        );
    }
    for _ in 0..(seconds * 0.8) as usize {
        let at = noise.next().abs() * (seconds - 0.2);
        click(
            &mut fire,
            at,
            500.0 + noise.next().abs() * 500.0,
            1.6,
            0.05,
            0.28,
            &mut noise,
        );
    }
    lowpass(&mut fire, 4800.0);
    for (o, f) in out.iter_mut().zip(&fire) {
        *o += f;
    }
    out
}

/// A stone hall's room tone with torches burning (a seamless stereo loop).
fn castle_bed() -> Vec<f32> {
    let seconds = 20.0;
    let (l, r) = (
        castle_channel(0xCA57_1E01, seconds),
        castle_channel(0xCA57_1E02, seconds),
    );
    let peak = l
        .iter()
        .chain(&r)
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.5 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        l.iter().map(|s| s * scale).collect(),
        r.iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

/// A field of crickets after dark (a seamless stereo loop).
fn crickets_bed() -> Vec<f32> {
    let seconds = 16.0;
    let n = (seconds * RATE as f32) as usize;
    let mut channels = Vec::new();
    for (seed, side) in [(0xC41C_0001u32, 0.0f32), (0xC41C_0002, 1.0)] {
        let mut noise = Noise(seed);
        let mut out = vec![0.0f32; n];
        for k in 0..6 {
            let carrier = 4100.0 + noise.next().abs() * 900.0 + side * 40.0;
            let chirp_rate = 2.4 + (k as f32) * 0.05 + noise.next().abs() * 0.05;
            let pulse_rate = 26.0 + noise.next().abs() * 5.0;
            let offset = noise.next().abs();
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                phase += std::f32::consts::TAU * carrier / RATE as f32;
                // Groups of pulses: on for a fraction of each chirp period.
                let cycle = (t * chirp_rate + offset).fract();
                let on = if cycle < 0.42 {
                    (cycle / 0.42 * std::f32::consts::PI).sin()
                } else {
                    0.0
                };
                let pulse = ((t * pulse_rate).fract() * std::f32::consts::TAU / 2.0)
                    .sin()
                    .max(0.0)
                    .powi(2);
                *s += phase.sin() * on * pulse * 0.16;
            }
        }
        lowpass(&mut out, 7000.0);
        channels.push(out);
    }
    let peak = channels
        .iter()
        .flatten()
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.4 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        channels[0].iter().map(|s| s * scale).collect(),
        channels[1].iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

/// A soft voice: a glottal buzz (harmonics falling off) gliding through a pitch
/// curve, shaped by vowel formants, with breath mixed in. `pitch(u)` and
/// `open(u)` (0 = hummed "mm", 1 = open "ah") take the syllable's 0..1 progress.
#[allow(clippy::too_many_arguments)]
fn voice(
    out: &mut [f32],
    at: f32,
    length: f32,
    pitch: impl Fn(f32) -> f32,
    open: impl Fn(f32) -> f32,
    breath: f32,
    gain: f32,
    noise: &mut Noise,
) {
    let start = (at * RATE as f32) as usize;
    let n = (length * RATE as f32) as usize;
    let mut raw = vec![0.0f32; n];
    let mut hum = vec![0.0f32; n];
    let mut phase = 0.0f32;
    for i in 0..n {
        let u = i as f32 / n.max(1) as f32;
        let f = pitch(u) * (1.0 + 0.012 * (t_of(i) * std::f32::consts::TAU * 5.5).sin());
        phase += std::f32::consts::TAU * f / RATE as f32;
        let mut buzz = 0.0;
        for h in 1..=14 {
            buzz += (phase * h as f32).sin() / (h as f32).powf(1.35);
        }
        let swell = (u * std::f32::consts::PI).sin().powf(0.8);
        raw[i] = (buzz * 0.5 + noise.next() * breath) * swell;
        hum[i] = raw[i];
    }
    // "ah": the first two formants of an open vowel; "mm": a closed, nasal hum.
    let mut f1 = raw.clone();
    bandpass(&mut f1, 820.0, 2.2);
    let mut f2 = raw.clone();
    bandpass(&mut f2, 1250.0, 3.0);
    let mut f3 = raw;
    bandpass(&mut f3, 2800.0, 4.0);
    lowpass(&mut hum, 420.0);
    for i in 0..n {
        let u = i as f32 / n.max(1) as f32;
        let o = open(u).clamp(0.0, 1.0);
        let s = (f1[i] * 1.0 + f2[i] * 0.55 + f3[i] * 0.18) * o + hum[i] * 1.6 * (1.0 - o);
        if let Some(d) = out.get_mut(start + i) {
            *d += s * gain;
        }
    }
}

/// Heard through a closed door: only the low of it, in a small room.
fn behind_door(mut signal: Vec<f32>) -> Vec<f32> {
    lowpass(&mut signal, 1300.0);
    lowpass(&mut signal, 1600.0);
    cave(signal, 0.10, 0.35, 0.0)
}

/// The brothel's air: a warm wooden room, the fire down the hall and voices
/// murmuring somewhere, never a word to be made out.
fn brothel_channel(seed: u32, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(seed);
    let mut room = brown(&mut noise, n);
    lowpass(&mut room, 260.0);
    let mut out: Vec<f32> = room.iter().map(|s| s * 2.0).collect();
    // Murmur: syllables of low voices, far off, never quite words.
    let mut talk = vec![0.0f32; n];
    let mut t = 0.3;
    while t < seconds - 1.0 {
        let base = if noise.next() > 0.0 { 150.0 } else { 230.0 };
        let syllables = 3 + (noise.next().abs() * 6.0) as usize;
        for k in 0..syllables {
            let len = 0.12 + noise.next().abs() * 0.14;
            let f = base * (1.0 + noise.next() * 0.12);
            let at = t + k as f32 * (len + 0.03);
            voice(
                &mut talk,
                at,
                len,
                |u| f * (1.0 - 0.08 * u),
                |u| 0.4 + 0.5 * (u * 3.0).sin().abs(),
                0.2,
                0.25,
                &mut noise,
            );
        }
        t += syllables as f32 * 0.22 + 0.6 + noise.next().abs() * 2.2;
    }
    lowpass(&mut talk, 900.0);
    let talk = cave(talk, 0.3, 0.6, 0.0);
    for (o, v) in out.iter_mut().zip(&talk) {
        *o += v * 0.35;
    }
    // The fire, far down the room: a slow crackle.
    for _ in 0..(seconds * 7.0) as usize {
        let at = noise.next().abs() * (seconds - 0.1);
        click(
            &mut out,
            at,
            700.0 + noise.next().abs() * 1600.0,
            2.0,
            0.012,
            0.03 + noise.next().abs() * 0.04,
            &mut noise,
        );
    }
    lowpass(&mut out, 5000.0);
    out
}

/// The brothel's room tone (a seamless stereo loop).
fn brothel_bed() -> Vec<f32> {
    let seconds = 24.0;
    let (l, r) = (
        brothel_channel(0xB207_E101, seconds),
        brothel_channel(0xB207_E102, seconds),
    );
    let peak = l
        .iter()
        .chain(&r)
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.5 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        l.iter().map(|s| s * scale).collect(),
        r.iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

fn springs_channel(seed: u32, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(seed);
    // The waterfalls: broad pink rush, its body low, its spray soft.
    let mut fall = brown(&mut noise, n);
    lowpass(&mut fall, 1400.0);
    let mut spray: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    bandpass(&mut spray, 2600.0, 0.7);
    // The streams: a bubbling murmur.
    let mut murmur: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    swept_bandpass(
        &mut murmur,
        |u| 700.0 + 300.0 * (u * std::f32::consts::TAU * 9.0).sin(),
        3.0,
    );
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let t = t_of(i);
        let surge =
            0.85 + 0.15 * (t * std::f32::consts::TAU / seconds * 3.0 + seed as f32 * 0.1).sin();
        out[i] = fall[i] * 2.2 * surge + spray[i] * 0.10 * surge + murmur[i] * 0.16;
    }
    // A bubble here and there in the pools.
    for _ in 0..(seconds * 2.5) as usize {
        let at = noise.next().abs() * (seconds - 0.2);
        let f = 500.0 + noise.next().abs() * 700.0;
        tone(&mut out, at, f, f * 1.6, 0.03, 0.001, 0.018, 0.08);
    }
    out
}

/// Waterfalls and bubbling pools (a seamless stereo loop).
fn springs_bed() -> Vec<f32> {
    let seconds = 20.0;
    let (l, r) = (
        springs_channel(0x5E1_0A51, seconds),
        springs_channel(0x5E1_0A52, seconds),
    );
    let peak = l
        .iter()
        .chain(&r)
        .fold(0.0f32, |m, s| m.max(s.abs()))
        .max(1e-6);
    let scale = 0.5 / peak;
    let (l, r): (Vec<f32>, Vec<f32>) = (
        l.iter().map(|s| s * scale).collect(),
        r.iter().map(|s| s * scale).collect(),
    );
    crossfade_loop(stereo(&l, &r))
}

/// A wood pigeon: soft, low, rhythmic.
fn dove(noise: &mut Noise) -> Vec<f32> {
    let _ = noise;
    let mut out = buffer(1.5);
    for (at, f, len) in [
        (0.0, 470.0, 0.28),
        (0.34, 540.0, 0.24),
        (0.66, 480.0, 0.16),
        (0.90, 470.0, 0.16),
        (1.12, 440.0, 0.2),
    ] {
        note(
            &mut out,
            at,
            f * 1.06,
            f * 0.94,
            len,
            0.7,
            (6.0, 22.0),
            0.15,
        );
    }
    lowpass(&mut out, 1400.0);
    cave(out, 0.04, 0.7, 0.22)
}

fn synth(amb: Amb) -> Vec<f32> {
    let mut noise = Noise(0xA3B1_C0DE ^ (amb as u32).wrapping_mul(0x9E37_79B1));
    let signal = match amb {
        Amb::FireCrackle => {
            // A handful of snaps and a soft hiss: logs settling in the grate.
            let mut out = buffer(0.9);
            for _ in 0..9 {
                let at = noise.next().abs() * 0.7;
                click(
                    &mut out,
                    at,
                    1200.0 + noise.next().abs() * 2600.0,
                    2.2,
                    0.01 + noise.next().abs() * 0.02,
                    0.2 + noise.next().abs() * 0.5,
                    &mut noise,
                );
            }
            click(&mut out, 0.3, 600.0, 1.4, 0.06, 0.6, &mut noise);
            for i in 0..out.len() {
                out[i] += noise.next() * 0.03 * env(t_of(i), 0.1, 0.5);
            }
            lowpass(&mut out, 6000.0);
            out
        }
        Amb::GlassClink => {
            // Two glasses touching: bright partials that ring and die.
            let mut out = buffer(1.0);
            for (at, gain) in [(0.0f32, 1.0f32), (0.16, 0.45)] {
                for (f, g) in [(2650.0, 1.0), (3970.0, 0.5), (5320.0, 0.3)] {
                    tone(&mut out, at, f, f * 0.998, 0.8, 0.001, 0.22, g * gain * 0.5);
                }
                click(&mut out, at, 4000.0, 2.0, 0.004, 0.4 * gain, &mut noise);
            }
            out
        }
        Amb::Pour => {
            // Wine into a glass: a gurgle that rises as it fills.
            let mut out: Vec<f32> = (0..(1.4 * RATE as f32) as usize)
                .map(|_| noise.next())
                .collect();
            swept_bandpass(
                &mut out,
                |u| 500.0 + 900.0 * u + 180.0 * (u * 70.0).sin(),
                6.0,
            );
            for (i, s) in out.iter_mut().enumerate() {
                *s *= env(t_of(i), 0.08, 1.2) * (0.7 + 0.3 * (t_of(i) * 31.0).sin().abs());
            }
            out
        }
        Amb::Quill => {
            // A pen scratching paper: short dry strokes.
            let mut out = buffer(1.2);
            let mut at = 0.02;
            while at < 1.0 {
                let len = 0.06 + noise.next().abs() * 0.12;
                let start = (at * RATE as f32) as usize;
                for i in 0..(len * RATE as f32) as usize {
                    if let Some(s) = out.get_mut(start + i) {
                        let u = i as f32 / (len * RATE as f32);
                        *s += noise.next() * (u * std::f32::consts::PI).sin() * 0.5;
                    }
                }
                at += len + 0.03 + noise.next().abs() * 0.08;
            }
            bandpass(&mut out, 3600.0, 1.2);
            out
        }
        Amb::Giggle => {
            // A soft laugh: quick "ha"s stepping down.
            let mut out = buffer(1.3);
            let f = 390.0 + noise.next().abs() * 60.0;
            for k in 0..5 {
                let at = 0.05 + k as f32 * 0.17;
                let fk = f * (1.0 - k as f32 * 0.04);
                voice(
                    &mut out,
                    at,
                    0.12,
                    move |u| fk * (1.04 - 0.1 * u),
                    |_| 0.9,
                    0.35,
                    0.8,
                    &mut noise,
                );
            }
            cave(out, 0.12, 0.4, 0.0)
        }
        Amb::Moan => {
            // A soft hummed "mm" that opens a little, through the door.
            let mut out = buffer(1.6);
            voice(
                &mut out,
                0.05,
                1.35,
                |u| 330.0 + 40.0 * (u * std::f32::consts::PI).sin() - 20.0 * u,
                |u| (u * 1.6 - 0.4).clamp(0.0, 0.7),
                0.25,
                1.0,
                &mut noise,
            );
            behind_door(out)
        }
        Amb::MoanRise => {
            // Two breaths rising into an "ah".
            let mut out = buffer(2.2);
            voice(
                &mut out,
                0.05,
                0.55,
                |u| 310.0 + 50.0 * u,
                |u| 0.2 + 0.5 * u,
                0.3,
                0.8,
                &mut noise,
            );
            voice(
                &mut out,
                0.8,
                1.2,
                |u| 350.0 + 90.0 * (u * 1.4).min(1.0) - 60.0 * u * u,
                |u| 0.5 + 0.5 * (u * std::f32::consts::PI).sin(),
                0.3,
                1.0,
                &mut noise,
            );
            behind_door(out)
        }
        Amb::Sigh => {
            // A long breath out, voiced only at its start.
            let mut out = buffer(1.5);
            voice(
                &mut out,
                0.05,
                1.3,
                |u| 300.0 - 70.0 * u,
                |u| 0.8 - 0.6 * u,
                0.9,
                0.9,
                &mut noise,
            );
            behind_door(out)
        }
        Amb::BedCreak => {
            // An old bed's frame, a few slow creaks.
            let mut out = buffer(2.6);
            for k in 0..4 {
                let at = 0.05 + k as f32 * (0.55 + noise.next().abs() * 0.1);
                let n = (0.32 * RATE as f32) as usize;
                let start = (at * RATE as f32) as usize;
                let mut creak: Vec<f32> = (0..n).map(|_| noise.next()).collect();
                let f = 380.0 + noise.next().abs() * 120.0;
                swept_bandpass(&mut creak, |u| f + 160.0 * u, 16.0);
                for (i, c) in creak.iter().enumerate() {
                    let u = i as f32 / n as f32;
                    // A rasp: the grain of the wood catching.
                    let rasp = 0.5 + 0.5 * (t_of(i) * std::f32::consts::TAU * 38.0).sin().abs();
                    if let Some(s) = out.get_mut(start + i) {
                        *s += c
                            * (u * std::f32::consts::PI).sin()
                            * rasp
                            * (0.8 + 0.2 * k as f32 / 4.0);
                    }
                }
            }
            behind_door(out)
        }
        Amb::BirdTweet => {
            let mut out = buffer(0.9);
            let base = 3000.0 + noise.next().abs() * 1200.0;
            for k in 0..4 {
                let at = 0.02 + k as f32 * 0.15;
                let lift = 1.0 + (k as f32 % 2.0) * 0.12;
                note(
                    &mut out,
                    at,
                    base * lift,
                    base * lift * 1.35,
                    0.075,
                    0.55,
                    (30.0, 60.0),
                    0.25,
                );
            }
            cave(out, 0.04, 0.7, 0.28)
        }
        Amb::BirdTrill => {
            let mut out = buffer(1.1);
            let base = 4300.0 + noise.next().abs() * 700.0;
            for k in 0..15 {
                let at = 0.03 + k as f32 * 0.048;
                let f = if k % 2 == 0 { base } else { base * 1.14 };
                note(
                    &mut out,
                    at,
                    f,
                    f * 1.03,
                    0.032,
                    0.5 * (1.0 - k as f32 / 40.0),
                    (0.0, 0.0),
                    0.2,
                );
            }
            cave(out, 0.04, 0.7, 0.28)
        }
        Amb::BirdSong => {
            let mut out = buffer(2.0);
            let scale = [1.0f32, 1.122, 1.26, 1.5, 1.68, 1.5, 1.26, 1.122];
            let base = 2500.0 + noise.next().abs() * 700.0;
            let mut at = 0.04;
            for (k, step) in scale.iter().enumerate() {
                let len = 0.09 + (k % 3) as f32 * 0.04;
                note(
                    &mut out,
                    at,
                    base * step,
                    base * step * (1.0 + 0.10 * (k % 2) as f32),
                    len,
                    0.5,
                    (18.0, 34.0),
                    0.3,
                );
                at += len + 0.055;
            }
            note(
                &mut out,
                at + 0.04,
                base * 1.9,
                base * 1.2,
                0.3,
                0.45,
                (25.0, 20.0),
                0.2,
            );
            cave(out, 0.05, 0.9, 0.3)
        }
        Amb::Dove => dove(&mut noise),
        Amb::Crow => {
            let mut out = buffer(1.6);
            for (at, f, len) in [(0.0, 430.0, 0.36), (0.58, 400.0, 0.4)] {
                let n = (len * RATE as f32) as usize;
                let mut caw = vec![0.0f32; n];
                let mut phase = 0.0f32;
                for (i, s) in caw.iter_mut().enumerate() {
                    let u = i as f32 / n as f32;
                    phase += std::f32::consts::TAU * (f * (1.0 - 0.28 * u)) / RATE as f32;
                    // A sawtooth-ish buzz, then formants shape the vowel.
                    let saw = (phase.sin()
                        + (phase * 2.0).sin() * 0.6
                        + (phase * 3.0).sin() * 0.4
                        + noise.next() * 0.35)
                        * (u * std::f32::consts::PI).sin().powf(0.5);
                    *s = saw;
                }
                let mut a = caw.clone();
                bandpass(&mut a, 900.0, 3.0);
                let mut b = caw;
                bandpass(&mut b, 1700.0, 4.0);
                let start = (at * RATE as f32) as usize;
                for (k, (x, y)) in a.iter().zip(&b).enumerate() {
                    if let Some(s) = out.get_mut(start + k) {
                        *s += (x + y * 0.7) * 1.4;
                    }
                }
            }
            cave(out, 0.05, 1.2, 0.4)
        }
        Amb::Woodpecker => {
            let mut out = buffer(1.5);
            for k in 0..13 {
                let at = 0.02 + k as f32 * 0.058;
                let g = 1.0 - k as f32 * 0.04;
                click(&mut out, at, 1500.0, 2.5, 0.02, 0.9 * g, &mut noise);
                tone(&mut out, at, 900.0, 600.0, 0.02, 0.0005, 0.008, 0.3 * g);
            }
            cave(out, 0.05, 0.9, 0.3)
        }
        Amb::Owl => {
            let mut out = buffer(2.6);
            for (at, f, len) in [(0.0, 390.0, 0.34), (0.55, 380.0, 0.30), (1.05, 370.0, 0.9)] {
                note(&mut out, at, f * 1.05, f * 0.93, len, 0.7, (4.0, 5.0), 0.12);
            }
            lowpass(&mut out, 1000.0);
            cave(out, 0.05, 1.4, 0.35)
        }
        Amb::BranchCreak => {
            let mut out = buffer(1.6);
            let n = out.len();
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                let u = i as f32 / n as f32;
                let f = 105.0 + 55.0 * u + 12.0 * (t * 23.0).sin();
                phase += std::f32::consts::TAU * f / RATE as f32;
                let stutter = 0.55 + 0.45 * (t * std::f32::consts::TAU * 11.0).sin().max(0.0);
                *s = (phase.sin() + (phase * 3.0).sin() * 0.5 + noise.next() * 0.25)
                    * stutter
                    * (u * std::f32::consts::PI).sin().powi(2);
            }
            bandpass(&mut out, 620.0, 1.4);
            cave(out, 0.05, 0.8, 0.2)
        }
        Amb::TwigSnap => {
            let mut out = buffer(0.5);
            click(&mut out, 0.0, 2300.0, 2.0, 0.03, 1.0, &mut noise);
            click(&mut out, 0.045, 1700.0, 2.0, 0.05, 0.5, &mut noise);
            tone(&mut out, 0.0, 1100.0, 700.0, 0.06, 0.0005, 0.02, 0.3);
            cave(out, 0.05, 0.6, 0.25)
        }
        Amb::LeafGust => {
            let n = (2.2 * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            highpass(&mut out, 1400.0);
            lowpass(&mut out, 6500.0);
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                *s *=
                    (u * std::f32::consts::PI).sin().powi(2) * (0.7 + 0.3 * (t_of(i) * 37.0).sin());
            }
            out
        }
        Amb::Hammer => {
            let mut out = buffer(2.2);
            for (at, g) in [
                (0.05, 1.0),
                (0.5, 0.9),
                (0.93, 0.95),
                (1.16, 0.7),
                (1.62, 1.0),
            ] {
                click(&mut out, at, 3200.0, 1.5, 0.02, 0.9 * g, &mut noise);
                for (f, gain) in [(1760.0, 0.5), (2394.0, 0.35), (3320.0, 0.25)] {
                    tone(&mut out, at, f, f * 0.995, 0.4, 0.0008, 0.09, gain * g);
                }
                tone(&mut out, at, 180.0, 130.0, 0.08, 0.001, 0.03, 0.4 * g);
            }
            cave(out, 0.06, 0.9, 0.3)
        }
        Amb::BellFar => {
            let mut out = buffer(4.5);
            for (ratio, g, decay) in [
                (1.0, 1.0, 1.8),
                (2.0, 0.55, 1.3),
                (2.76, 0.35, 0.9),
                (5.4, 0.18, 0.5),
                (0.5, 0.4, 2.2),
            ] {
                tone(
                    &mut out,
                    0.0,
                    392.0 * ratio,
                    392.0 * ratio,
                    4.2,
                    0.004,
                    decay,
                    g,
                );
            }
            lowpass(&mut out, 2600.0);
            cave(out, 0.10, 1.4, 0.35)
        }
        Amb::DoorCreak => {
            let mut out = buffer(1.5);
            let n = out.len();
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                let f = 170.0 + 120.0 * u + 9.0 * (t_of(i) * 31.0).sin();
                phase += std::f32::consts::TAU * f / RATE as f32;
                let saw = phase.sin()
                    + (phase * 2.0).sin() * 0.7
                    + (phase * 3.0).sin() * 0.5
                    + (phase * 4.0).sin() * 0.35;
                *s = (saw + noise.next() * 0.2) * (u * std::f32::consts::PI).sin().powf(0.8);
            }
            let mut a = out.clone();
            bandpass(&mut a, 780.0, 3.5);
            let mut b = out;
            bandpass(&mut b, 1450.0, 4.0);
            let mixed: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + y * 0.6).collect();
            cave(mixed, 0.08, 0.7, 0.0)
        }
        Amb::Bubble => {
            // A few bubbles rising and breaking: little rising blips, soft.
            let mut out = buffer(0.7);
            for k in 0..(3 + (noise.next().abs() * 4.0) as usize) {
                let at = k as f32 * 0.07 + noise.next().abs() * 0.12;
                let f = 520.0 + noise.next().abs() * 600.0;
                tone(
                    &mut out,
                    at,
                    f,
                    f * 1.7,
                    0.035,
                    0.001,
                    0.02,
                    0.35 + noise.next().abs() * 0.3,
                );
            }
            lowpass(&mut out, 3200.0);
            out
        }
        Amb::Slosh => {
            // Someone shifting in the water: a low wash and a lap at the stones.
            let mut out = buffer(1.4);
            let mut wash: Vec<f32> = (0..out.len()).map(|_| noise.next()).collect();
            swept_bandpass(&mut wash, |u| 500.0 - 260.0 * u, 2.5);
            for (i, (o, w)) in out.iter_mut().zip(&wash).enumerate() {
                let t = t_of(i);
                *o += w * env(t, 0.12, 0.9) * 0.5;
            }
            for _ in 0..8 {
                let at = 0.3 + noise.next().abs() * 0.8;
                let f = 300.0 + noise.next().abs() * 400.0;
                tone(&mut out, at, f, f * 1.5, 0.04, 0.002, 0.025, 0.2);
            }
            lowpass(&mut out, 2400.0);
            out
        }
        Amb::Clack => {
            // The bamboo deer-scarer tipping onto its stone: a hollow clack and its ring.
            let mut out = buffer(0.9);
            click(&mut out, 0.0, 1400.0, 1.2, 0.02, 1.0, &mut noise);
            tone(&mut out, 0.0, 620.0, 600.0, 0.35, 0.001, 0.25, 0.7);
            tone(&mut out, 0.0, 1240.0, 1230.0, 0.2, 0.001, 0.12, 0.25);
            cave(out, 0.25, 0.6, 0.0)
        }
        Amb::Splash => {
            let mut out = buffer(1.8);
            for _ in 0..30 {
                let at = noise.next().abs() * 1.5;
                let f = 380.0 + noise.next().abs() * 700.0;
                tone(
                    &mut out,
                    at,
                    f,
                    f * 1.9,
                    0.03 + noise.next().abs() * 0.03,
                    0.002,
                    0.012,
                    0.3 + noise.next().abs() * 0.4,
                );
            }
            let mut spray: Vec<f32> = (0..out.len()).map(|_| noise.next()).collect();
            highpass(&mut spray, 1600.0);
            lowpass(&mut spray, 6000.0);
            for (i, (o, s)) in out.iter_mut().zip(&spray).enumerate() {
                *o += s
                    * 0.14
                    * (t_of(i) / 1.7 * std::f32::consts::PI)
                        .sin()
                        .max(0.0)
                        .powi(2);
            }
            out
        }
        Amb::Chime => {
            let mut out = buffer(3.2);
            let notes = [1568.0f32, 1760.0, 2093.0, 2349.0, 2637.0];
            for k in 0..5 {
                let at = noise.next().abs() * 1.4;
                let f = notes[(noise.next().abs() * notes.len() as f32) as usize % notes.len()];
                tone(&mut out, at, f, f, 2.0, 0.001, 0.55, 0.5 - k as f32 * 0.05);
                tone(&mut out, at, f * 2.76, f * 2.76, 0.8, 0.001, 0.16, 0.12);
            }
            cave(out, 0.05, 1.0, 0.25)
        }
        Amb::TorchWhoosh => {
            let n = (1.3 * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            lowpass(&mut out, 1500.0);
            swept_bandpass(
                &mut out,
                |u| 300.0 + 500.0 * (u * std::f32::consts::PI).sin(),
                0.9,
            );
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                *s *= (u.powf(0.6) * std::f32::consts::PI).sin().powi(2);
            }
            for k in 0..14 {
                click(
                    &mut out,
                    0.1 + k as f32 * 0.07 + noise.next().abs() * 0.03,
                    1200.0 + noise.next().abs() * 2000.0,
                    2.0,
                    0.02,
                    0.12,
                    &mut noise,
                );
            }
            cave(out, 0.25, 0.9, 0.0)
        }
        Amb::StoneCreak => {
            let mut out = buffer(2.0);
            let n = out.len();
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                let f = 68.0 + 30.0 * u + 3.0 * (t_of(i) * 17.0).sin();
                phase += std::f32::consts::TAU * f / RATE as f32;
                let wobble = 0.6 + 0.4 * (t_of(i) * std::f32::consts::TAU * 8.0).sin();
                *s = (phase.sin() + (phase * 3.0).sin() * 0.4 + noise.next() * 0.2)
                    * wobble
                    * (u * std::f32::consts::PI).sin().powi(2);
            }
            lowpass(&mut out, 900.0);
            cave(out, 0.35, 1.4, 0.0)
        }
        Amb::Chain => {
            let mut out = buffer(2.0);
            let mut at = 0.05;
            for _ in 0..8 {
                let f = 1800.0 + noise.next().abs() * 1700.0;
                click(&mut out, at, f, 3.0, 0.02, 0.6, &mut noise);
                tone(&mut out, at, f, f * 0.99, 0.12, 0.0006, 0.035, 0.4);
                at += 0.09 + noise.next().abs() * 0.16;
            }
            cave(out, 0.30, 1.1, 0.0)
        }
        Amb::Cloth => {
            let n = (0.7 * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            highpass(&mut out, 600.0);
            lowpass(&mut out, 3000.0);
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                *s *= (u * std::f32::consts::PI).sin().powi(3);
            }
            cave(out, 0.12, 0.6, 0.0)
        }
        Amb::WindHowl => {
            let n = (4.0 * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            swept_bandpass(
                &mut out,
                |u| 300.0 + 220.0 * (u * std::f32::consts::PI).sin() + 60.0 * (u * 19.0).sin(),
                11.0,
            );
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                *s *= (u * std::f32::consts::PI).sin().powi(2);
            }
            cave(out, 0.2, 1.4, 0.0)
        }
        Amb::FarDoor => {
            let mut out = buffer(2.4);
            tone(&mut out, 0.0, 70.0, 38.0, 0.5, 0.002, 0.14, 1.0);
            let mut thud = buffer(2.4);
            for (i, s) in thud.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.001, 0.05);
            }
            lowpass(&mut thud, 420.0);
            for (o, t) in out.iter_mut().zip(&thud) {
                *o += t * 0.8;
            }
            cave(out, 0.6, 1.8, 0.35)
        }
        Amb::CandlePop => {
            let mut out = buffer(0.4);
            click(&mut out, 0.0, 1600.0, 2.0, 0.02, 1.0, &mut noise);
            click(&mut out, 0.05, 900.0, 2.0, 0.03, 0.4, &mut noise);
            tone(&mut out, 0.0, 700.0, 500.0, 0.05, 0.001, 0.015, 0.4);
            cave(out, 0.2, 0.6, 0.0)
        }
    };
    normalise(signal, 0.9)
}

// ---------------------------------------------------------------- footsteps

/// What the ground under the Queen sounds like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ground {
    Grass,
    Cobble,
    Hall,
    Wood,
    Water,
    Stone,
}

impl Ground {
    pub(crate) fn of(zone: Zone) -> Self {
        match zone {
            Zone::Forest => Ground::Grass,
            Zone::Village => Ground::Cobble,
            Zone::Castle | Zone::Cave => Ground::Hall,
            Zone::Brothel => Ground::Wood,
        }
    }
    pub(crate) fn sfx(self) -> Sfx {
        match self {
            Ground::Grass => Sfx::StepGrass,
            Ground::Cobble => Sfx::StepCobble,
            Ground::Hall => Sfx::StepHall,
            Ground::Wood => Sfx::StepWood,
            Ground::Water => Sfx::StepWater,
            Ground::Stone => Sfx::StepCobble,
        }
    }
}

/// The synthesised footsteps (called from the effect bank).
pub(crate) fn step(sfx: Sfx, noise: &mut Noise) -> Vec<f32> {
    match sfx {
        Sfx::StepGrass => {
            // Blades and soil: a soft brush with a faint low thump.
            let n = (0.22 * RATE as f32) as usize;
            let mut out = vec![0.0f32; n];
            for (at, g) in [(0.0f32, 1.0f32), (0.055, 0.6)] {
                let start = (at * RATE as f32) as usize;
                for i in 0..(0.12 * RATE as f32) as usize {
                    if let Some(s) = out.get_mut(start + i) {
                        *s += noise.next() * env(t_of(i), 0.012, 0.045) * g;
                    }
                }
            }
            bandpass(&mut out, 1500.0, 0.6);
            lowpass(&mut out, 4200.0);
            tone(&mut out, 0.0, 120.0, 70.0, 0.1, 0.004, 0.03, 0.35);
            for k in 0..5 {
                click(
                    &mut out,
                    0.01 + k as f32 * 0.02,
                    3800.0,
                    1.2,
                    0.01,
                    0.05,
                    noise,
                );
            }
            out
        }
        Sfx::StepCobble => {
            // Hard sole on stone: a tap, a body and a grain of grit.
            let mut out = buffer(0.24);
            click(&mut out, 0.0, 2500.0, 2.0, 0.03, 0.9, noise);
            tone(&mut out, 0.0, 230.0, 140.0, 0.08, 0.001, 0.028, 0.7);
            for k in 0..8 {
                click(
                    &mut out,
                    0.012 + k as f32 * 0.012,
                    3200.0 + noise.next().abs() * 1500.0,
                    1.5,
                    0.006,
                    0.07,
                    noise,
                );
            }
            click(&mut out, 0.085, 1900.0, 2.0, 0.025, 0.35, noise);
            lowpass(&mut out, 5200.0);
            out
        }
        Sfx::StepWood => {
            // A heel on floorboards: a hollow knock and the board giving.
            let mut out = buffer(0.24);
            tone(&mut out, 0.0, 140.0, 105.0, 0.12, 0.002, 0.05, 0.9);
            click(&mut out, 0.0, 900.0, 1.4, 0.03, 0.5, noise);
            let mut board: Vec<f32> = (0..(0.14 * RATE as f32) as usize)
                .map(|_| noise.next())
                .collect();
            swept_bandpass(&mut board, |u| 420.0 + 120.0 * u, 12.0);
            for (i, b) in board.iter().enumerate() {
                if let Some(s) = out.get_mut((0.04 * RATE as f32) as usize + i) {
                    *s += b * env(t_of(i), 0.01, 0.08) * 0.12;
                }
            }
            cave(out, 0.12, 0.3, 0.0)
        }
        Sfx::StepWater => {
            // Wading: the leg pushing through, the water closing behind, a drip or two.
            let mut out = buffer(0.5);
            let mut push: Vec<f32> = (0..out.len()).map(|_| noise.next()).collect();
            swept_bandpass(&mut push, |u| 900.0 - 500.0 * u, 2.2);
            for (i, (o, p)) in out.iter_mut().zip(&push).enumerate() {
                *o += p * env(t_of(i), 0.03, 0.28) * 0.7;
            }
            for k in 0..4 {
                let at = 0.08 + k as f32 * 0.07 + noise.next().abs() * 0.05;
                let f = 450.0 + noise.next().abs() * 500.0;
                tone(&mut out, at, f, f * 1.8, 0.03, 0.001, 0.015, 0.25);
            }
            lowpass(&mut out, 3600.0);
            out
        }
        _ => {
            // A hall's flagstone: a round heel and the room answering.
            let mut out = buffer(0.2);
            tone(&mut out, 0.0, 165.0, 85.0, 0.1, 0.0015, 0.035, 0.9);
            click(&mut out, 0.0, 1500.0, 1.6, 0.03, 0.6, noise);
            click(&mut out, 0.07, 1100.0, 1.6, 0.03, 0.25, noise);
            cave(out, 0.32, 0.55, 0.0)
        }
    }
}

// ----------------------------------------------------------------- runtime

/// How one one-shot is scheduled in a zone.
struct Event {
    amb: Amb,
    /// Seconds between chances, min and max.
    every: (f32, f32),
    /// Chance to sound by day and by night (0..1).
    chance: (f32, f32),
    /// Loudness (0..1) and how far it is, in metres.
    volume: f32,
    distance: (f32, f32),
}

const fn ev(
    amb: Amb,
    min: f32,
    max: f32,
    day: f32,
    night: f32,
    volume: f32,
    near: f32,
    far: f32,
) -> Event {
    Event {
        amb,
        every: (min, max),
        chance: (day, night),
        volume,
        distance: (near, far),
    }
}

const FOREST: [Event; 10] = [
    ev(Amb::BirdTweet, 3.0, 8.0, 1.0, 0.0, 0.55, 6.0, 15.0),
    ev(Amb::BirdTrill, 7.0, 15.0, 1.0, 0.0, 0.5, 8.0, 18.0),
    ev(Amb::BirdSong, 9.0, 20.0, 1.0, 0.05, 0.55, 8.0, 20.0),
    ev(Amb::Dove, 16.0, 34.0, 0.9, 0.2, 0.5, 8.0, 18.0),
    ev(Amb::Crow, 28.0, 60.0, 0.7, 0.9, 0.55, 10.0, 24.0),
    ev(Amb::Woodpecker, 24.0, 55.0, 0.9, 0.0, 0.5, 9.0, 20.0),
    ev(Amb::Owl, 12.0, 26.0, 0.0, 1.0, 0.6, 9.0, 22.0),
    ev(Amb::BranchCreak, 9.0, 21.0, 0.8, 1.0, 0.55, 4.0, 11.0),
    ev(Amb::TwigSnap, 14.0, 38.0, 0.8, 1.0, 0.5, 3.0, 12.0),
    ev(Amb::LeafGust, 9.0, 19.0, 1.0, 1.0, 0.5, 3.0, 9.0),
];

const VILLAGE: [Event; 10] = [
    ev(Amb::BirdTweet, 5.0, 12.0, 1.0, 0.0, 0.45, 6.0, 16.0),
    ev(Amb::BirdSong, 12.0, 26.0, 0.9, 0.0, 0.45, 8.0, 20.0),
    ev(Amb::Dove, 20.0, 40.0, 0.8, 0.1, 0.4, 8.0, 20.0),
    ev(Amb::Hammer, 9.0, 22.0, 1.0, 0.05, 0.5, 9.0, 22.0),
    ev(Amb::BellFar, 45.0, 95.0, 0.8, 0.8, 0.45, 20.0, 40.0),
    ev(Amb::DoorCreak, 18.0, 45.0, 0.9, 0.7, 0.5, 5.0, 14.0),
    ev(Amb::Splash, 11.0, 24.0, 1.0, 0.7, 0.4, 5.0, 12.0),
    ev(Amb::Chime, 18.0, 42.0, 0.9, 1.0, 0.4, 5.0, 12.0),
    ev(Amb::Owl, 20.0, 45.0, 0.0, 0.5, 0.45, 12.0, 26.0),
    ev(Amb::Cloth, 20.0, 50.0, 0.7, 0.5, 0.3, 3.0, 8.0),
];

const CASTLE: [Event; 7] = [
    ev(Amb::TorchWhoosh, 14.0, 32.0, 1.0, 1.0, 0.45, 3.0, 8.0),
    ev(Amb::StoneCreak, 18.0, 40.0, 1.0, 1.0, 0.5, 4.0, 10.0),
    ev(Amb::Chain, 30.0, 70.0, 0.8, 0.9, 0.4, 5.0, 12.0),
    ev(Amb::Cloth, 20.0, 46.0, 1.0, 1.0, 0.3, 3.0, 8.0),
    ev(Amb::WindHowl, 24.0, 55.0, 0.8, 1.0, 0.4, 6.0, 14.0),
    ev(Amb::FarDoor, 60.0, 130.0, 0.9, 1.0, 0.4, 8.0, 18.0),
    ev(Amb::CandlePop, 8.0, 20.0, 1.0, 1.0, 0.35, 2.0, 6.0),
];

fn schedule(zone: Zone) -> &'static [Event] {
    match zone {
        Zone::Forest => &FOREST,
        Zone::Village => &VILLAGE,
        Zone::Castle => &CASTLE,
        Zone::Cave | Zone::Brothel => &[],
    }
}

/// The springs' own one-shots (a bird or two, the wind in the pines).
const SPRINGS: [Event; 4] = [
    ev(Amb::BirdTweet, 6.0, 14.0, 1.0, 0.0, 0.4, 7.0, 18.0),
    ev(Amb::BirdSong, 14.0, 30.0, 0.9, 0.0, 0.4, 9.0, 22.0),
    ev(Amb::LeafGust, 12.0, 26.0, 1.0, 1.0, 0.35, 5.0, 12.0),
    ev(Amb::Owl, 22.0, 45.0, 0.0, 0.7, 0.4, 12.0, 26.0),
];

fn schedule_for(scene: &Scene) -> &'static [Event] {
    if scene.springs > 0.5 {
        &SPRINGS
    } else {
        schedule(scene.zone)
    }
}

struct Bed {
    sink: Sink,
    level: f32,
}

/// A sound with a place: it sounds from `at` (world metres) now and then, loud
/// within `radius` and fading past it, softer from the other floor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Emitter {
    pub amb: Amb,
    pub at: Vec3,
    /// Seconds between times it sounds, min and max.
    pub every: (f32, f32),
    pub volume: f32,
    pub radius: f32,
    /// 0 ground floor, 1 upper floor (a listener on the other hears it faintly).
    pub level: u8,
}

/// Where the listener is and what the world is doing.
#[derive(Clone, Copy)]
pub(crate) struct Scene {
    pub zone: Zone,
    /// Sounds placed in the world around the listener (the brothel's).
    pub emitters: &'static [Emitter],
    /// 0 by day, 1 in the dead of night.
    pub night: f32,
    pub listener: Vec3,
    pub forward: Vec3,
    pub paused: bool,
    /// At the hot springs: 1 in the valley, a little in their rooms, 0 elsewhere.
    pub springs: f32,
}

/// The ambience's sounds, made ahead of time.
pub(crate) struct Data {
    bank: BTreeMap<Amb, Vec<f32>>,
    beds: Vec<(&'static str, Vec<f32>)>,
}

impl Data {
    pub(crate) fn build() -> Self {
        Self {
            bank: Amb::ALL.into_iter().map(|a| (a, synth(a))).collect(),
            beds: vec![
                ("forest", forest_bed()),
                ("village", village_bed()),
                ("castle", castle_bed()),
                ("crickets", crickets_bed()),
                ("brothel", brothel_bed()),
                ("springs", springs_bed()),
            ],
        }
    }
}

pub(crate) struct Ambience {
    bank: BTreeMap<Amb, Vec<f32>>,
    beds: BTreeMap<&'static str, Bed>,
    timers: BTreeMap<Amb, f32>,
    /// One timer per placed emitter, reset when the set of emitters changes.
    emitter_timers: Vec<f32>,
    emitters_of: usize,
    noise: Noise,
    zone: Option<Zone>,
}

impl Ambience {
    pub(crate) fn new(stream: &OutputStream, data: Data) -> Self {
        let mut beds = BTreeMap::new();
        for (name, samples) in data.beds {
            let sink = Sink::connect_new(stream.mixer());
            sink.append(SamplesBuffer::new(2, RATE, samples).repeat_infinite());
            sink.set_volume(0.0);
            beds.insert(name, Bed { sink, level: 0.0 });
        }
        Self {
            bank: data.bank,
            beds,
            timers: BTreeMap::new(),
            emitter_timers: Vec::new(),
            emitters_of: 0,
            noise: Noise(0xA3B1_E17C),
            zone: None,
        }
    }

    /// The level each bed should have in this scene (0..1 of its own gain).
    pub(crate) fn bed_targets(scene: &Scene) -> [(&'static str, f32); 6] {
        let outdoors = matches!(scene.zone, Zone::Forest | Zone::Village);
        let night = scene.night.clamp(0.0, 1.0);
        // At the springs (the villa's music goes on): their water instead of the villa's murmur,
        // muffled indoors.
        let springs = scene.springs;
        [
            (
                "forest",
                if scene.zone == Zone::Forest {
                    0.30
                } else {
                    0.0
                },
            ),
            (
                "village",
                if scene.zone == Zone::Village && springs == 0.0 {
                    0.26
                } else {
                    0.0
                },
            ),
            (
                "castle",
                if scene.zone == Zone::Castle {
                    0.22
                } else {
                    0.0
                },
            ),
            (
                "crickets",
                if outdoors {
                    night
                        * if scene.zone == Zone::Forest {
                            0.20
                        } else {
                            0.14
                        }
                } else {
                    0.0
                },
            ),
            (
                "brothel",
                if scene.zone == Zone::Brothel {
                    0.26
                } else {
                    0.0
                },
            ),
            ("springs", springs * 0.34),
        ]
    }

    /// Lowers the beds at once (the pause menu) or lets them back up.
    pub(crate) fn hush(&mut self, paused: bool) {
        for bed in self.beds.values() {
            bed.sink.set_volume(
                bed.level * if paused { 0.3 } else { 1.0 } * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Ambience),
            );
        }
    }

    pub(crate) fn update(&mut self, mixer: &rodio::mixer::Mixer, scene: &Scene, dt: f32) {
        let dt = dt.clamp(0.0, 0.2);
        let hush = if scene.paused { 0.3 } else { 1.0 } * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Ambience);
        for (name, target) in Self::bed_targets(scene) {
            if let Some(bed) = self.beds.get_mut(name) {
                bed.level += (target - bed.level) * (dt * 0.6).min(1.0);
                bed.sink.set_volume(bed.level * hush);
            }
        }
        if self.zone != Some(scene.zone) {
            // A new place starts its timers fresh, with the first sounds soon.
            self.zone = Some(scene.zone);
            self.timers.clear();
            for e in schedule(scene.zone) {
                let first = e.every.0 * 0.3 + (self.noise.next() * 0.5 + 0.5) * e.every.0;
                self.timers.insert(e.amb, first);
            }
        }
        if scene.paused {
            return;
        }
        self.update_emitters(mixer, scene, dt);
        for e in schedule_for(scene) {
            let timer = self.timers.entry(e.amb).or_insert(e.every.1);
            *timer -= dt;
            if *timer > 0.0 {
                continue;
            }
            *timer = e.every.0 + (self.noise.next() * 0.5 + 0.5) * (e.every.1 - e.every.0);
            let chance = e.chance.0 + (e.chance.1 - e.chance.0) * scene.night.clamp(0.0, 1.0);
            if (self.noise.next() * 0.5 + 0.5) < chance {
                self.play(mixer, e, scene);
            }
        }
    }

    /// Placed sounds: each emitter's timer runs down; when it sounds it is heard
    /// from where it is, by distance and side, and faintly from the other floor.
    fn update_emitters(&mut self, mixer: &rodio::mixer::Mixer, scene: &Scene, dt: f32) {
        if self.emitters_of != scene.emitters.as_ptr() as usize
            || self.emitter_timers.len() != scene.emitters.len()
        {
            self.emitters_of = scene.emitters.as_ptr() as usize;
            self.emitter_timers = scene
                .emitters
                .iter()
                .map(|e| e.every.0 * (0.2 + 0.6 * (self.noise.next() * 0.5 + 0.5)))
                .collect();
        }
        let listener_level = (scene.listener.y > 1.8) as u8;
        for (i, e) in scene.emitters.iter().enumerate() {
            self.emitter_timers[i] -= dt;
            if self.emitter_timers[i] > 0.0 {
                continue;
            }
            self.emitter_timers[i] =
                e.every.0 + (self.noise.next() * 0.5 + 0.5) * (e.every.1 - e.every.0);
            let to = e.at - scene.listener;
            let distance = Vec3::new(to.x, 0.0, to.z).length();
            let mut gain = 1.0 / (1.0 + (distance / e.radius.max(0.5)).powf(1.6));
            if e.level != listener_level {
                gain *= 0.22;
            }
            let volume = (e.volume * gain).min(1.0);
            if volume < 0.015 {
                continue;
            }
            // Which side: the source against the listener's right hand.
            let right = Vec3::new(-scene.forward.z, 0.0, scene.forward.x).normalize_or_zero();
            let flat = Vec3::new(to.x, 0.0, to.z);
            let pan = if flat.length() > 0.3 {
                flat.normalize_or_zero().dot(right).clamp(-1.0, 1.0) * 0.8
            } else {
                0.0
            };
            let Some(clip) = self.bank.get(&e.amb) else {
                continue;
            };
            let (left, right_gain) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
            let speed = 0.95 + (self.noise.next() * 0.5 + 0.5) * 0.1;
            let mut samples = Vec::with_capacity(clip.len() * 2);
            for s in clip {
                samples.push(s * left);
                samples.push(s * right_gain);
            }
            let sink = Sink::connect_new(mixer);
            sink.append(SamplesBuffer::new(2, RATE, samples).speed(speed));
            sink.set_volume(
                volume * if scene.paused { 0.0 } else { 1.0 } * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Ambience),
            );
            sink.detach();
        }
    }

    fn play(&mut self, mixer: &rodio::mixer::Mixer, e: &Event, scene: &Scene) {
        let Some(clip) = self.bank.get(&e.amb) else {
            return;
        };
        let azimuth = (self.noise.next() * 0.5 + 0.5) * std::f32::consts::TAU;
        let distance =
            e.distance.0 + (self.noise.next() * 0.5 + 0.5) * (e.distance.1 - e.distance.0);
        let gain = 1.0 / (1.0 + (distance / 9.0).powf(1.3));
        let (left, right) = place(azimuth);
        let volume = (e.volume * gain * 1.5).min(1.0);
        let speed = if matches!(e.amb, Amb::BellFar | Amb::FarDoor | Amb::Hammer | Amb::Owl) {
            1.0
        } else {
            0.93 + (self.noise.next() * 0.5 + 0.5) * 0.14
        };
        let _ = scene;
        let mut samples = Vec::with_capacity(clip.len() * 2);
        for s in clip {
            samples.push(s * left);
            samples.push(s * right);
        }
        let sink = Sink::connect_new(mixer);
        sink.append(SamplesBuffer::new(2, RATE, samples).speed(speed));
        sink.set_volume(volume * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Ambience));
        sink.detach();
    }
}

/// Every ambient one-shot, and each bed's left channel, as mono clips named for
/// export (`amb_<sound>`, `bed_<place>`).
pub(crate) fn export_clips() -> Vec<(String, Vec<f32>)> {
    let shots = Amb::ALL
        .into_iter()
        .map(|a| (format!("amb_{}", format!("{a:?}").to_lowercase()), synth(a)));
    let data = Data::build();
    let beds = data.beds.into_iter().map(|(name, stereo)| {
        (
            format!("bed_{name}"),
            stereo.iter().step_by(2).copied().collect(),
        )
    });
    shots.chain(beds).collect()
}

/// Left and right gain for a source at `azimuth` around the listener
/// (0 = straight ahead, growing to the right), keeping some sound in both
/// ears.
pub(crate) fn place(azimuth: f32) -> (f32, f32) {
    let pan = azimuth.sin() * 0.8;
    ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ambient_sound_is_finite_audible_and_bounded() {
        for amb in Amb::ALL {
            let clip = synth(amb);
            assert!(clip.len() > 2_000, "{amb:?} has length");
            assert!(
                clip.iter().all(|s| s.is_finite() && s.abs() <= 1.0),
                "{amb:?} stays in range"
            );
            let peak = clip.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.5, "{amb:?} is audible ({peak})");
            // Nothing starts or ends with a click.
            assert!(
                clip[0].abs() < 0.2 && clip[clip.len() - 1].abs() < 0.05,
                "{amb:?} starts and ends softly"
            );
        }
    }

    #[test]
    fn the_beds_are_stereo_seamless_and_quiet_enough_to_sit_under_music() {
        for (name, bed) in [
            ("forest", forest_bed()),
            ("village", village_bed()),
            ("castle", castle_bed()),
            ("crickets", crickets_bed()),
        ] {
            assert_eq!(bed.len() % 2, 0, "{name} is interleaved stereo");
            assert!(
                bed.iter().all(|s| s.is_finite() && s.abs() <= 1.0),
                "{name} in range"
            );
            let (a, b) = (bed[0], bed[bed.len() - 2]);
            assert!(
                (a - b).abs() < 0.3,
                "{name} loops without a jump ({a} vs {b})"
            );
            let peak = bed.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.15 && peak < 0.9, "{name} peak {peak}");
            // The two ears are not identical: the bed is wide.
            let differing = bed
                .chunks_exact(2)
                .filter(|f| (f[0] - f[1]).abs() > 0.01)
                .count();
            assert!(differing > bed.len() / 8, "{name} has stereo width");
        }
    }

    #[test]
    fn synthesising_all_the_ambience_stays_quick_enough_for_startup() {
        let started = std::time::Instant::now();
        for amb in Amb::ALL {
            let _ = synth(amb);
        }
        let _ = (forest_bed(), village_bed(), castle_bed(), crickets_bed());
        let ms = started.elapsed().as_millis();
        eprintln!("[ambience] synthesis took {ms} ms");
        assert!(ms < 6_000, "ambience synthesis took {ms} ms");
    }

    #[test]
    fn footsteps_of_each_ground_are_short_distinct_and_clean() {
        let mut noise = Noise(7);
        let mut peaks = Vec::new();
        for sfx in [Sfx::StepGrass, Sfx::StepCobble, Sfx::StepHall] {
            let clip = normalise(step(sfx, &mut noise), 0.9);
            assert!(clip.len() < RATE as usize, "{sfx:?} is short");
            assert!(clip.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
            // Brightness: share of energy in fast changes tells grass from stone.
            let fast: f32 =
                clip.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / clip.len() as f32;
            peaks.push(fast);
        }
        assert!(
            peaks[1] > peaks[2],
            "cobble is brighter than a hall's round heel: {peaks:?}"
        );
        assert_eq!(Ground::of(Zone::Forest).sfx(), Sfx::StepGrass);
        assert_eq!(Ground::of(Zone::Village).sfx(), Sfx::StepCobble);
        assert_eq!(Ground::of(Zone::Castle).sfx(), Sfx::StepHall);
    }

    #[test]
    fn beds_follow_the_zone_and_crickets_follow_the_night() {
        let scene = |zone, night| Scene {
            emitters: &[],
            zone,
            night,
            listener: Vec3::ZERO,
            forward: Vec3::Z,
            paused: false,
            springs: 0.0,
        };
        let level = |s: &Scene, name: &str| {
            Ambience::bed_targets(s)
                .into_iter()
                .find(|b| b.0 == name)
                .unwrap()
                .1
        };
        assert!(level(&scene(Zone::Forest, 0.0), "forest") > 0.0);
        assert_eq!(level(&scene(Zone::Village, 0.0), "forest"), 0.0);
        assert!(level(&scene(Zone::Village, 0.0), "village") > 0.0);
        assert!(level(&scene(Zone::Castle, 0.0), "castle") > 0.0);
        assert_eq!(level(&scene(Zone::Cave, 0.0), "castle"), 0.0);
        assert_eq!(
            level(&scene(Zone::Forest, 0.0), "crickets"),
            0.0,
            "no crickets by day"
        );
        assert!(
            level(&scene(Zone::Forest, 1.0), "crickets")
                > level(&scene(Zone::Village, 1.0), "crickets")
        );
        assert_eq!(
            level(&scene(Zone::Castle, 1.0), "crickets"),
            0.0,
            "none indoors"
        );
    }

    #[test]
    fn every_zone_schedule_is_sane_and_night_and_day_differ() {
        for zone in [Zone::Forest, Zone::Village, Zone::Castle] {
            let events = schedule(zone);
            assert!(events.len() >= 6, "{zone:?} has variety");
            for e in events {
                assert!(e.every.0 > 0.0 && e.every.0 < e.every.1, "{:?}", e.amb);
                assert!((0.0..=1.0).contains(&e.chance.0) && (0.0..=1.0).contains(&e.chance.1));
                assert!(e.volume > 0.0 && e.volume <= 1.0);
                assert!(e.distance.0 < e.distance.1);
            }
        }
        let forest = schedule(Zone::Forest);
        let owl = forest.iter().find(|e| e.amb == Amb::Owl).unwrap();
        let tweet = forest.iter().find(|e| e.amb == Amb::BirdTweet).unwrap();
        assert!(owl.chance.1 > owl.chance.0, "owls are for the night");
        assert!(tweet.chance.0 > tweet.chance.1, "small birds sing by day");
        assert!(
            schedule(Zone::Cave).is_empty(),
            "the cave keeps its own drips"
        );
    }

    #[test]
    fn panning_keeps_both_ears_and_leans_toward_the_source() {
        let (l, r) = place(std::f32::consts::FRAC_PI_2); // hard right
        assert!(r > l && l > 0.15);
        let (l, r) = place(-std::f32::consts::FRAC_PI_2);
        assert!(l > r && r > 0.15);
        let (l, r) = place(0.0);
        assert!((l - r).abs() < 1e-3);
    }
}
