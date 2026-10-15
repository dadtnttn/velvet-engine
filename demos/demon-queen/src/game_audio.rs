//! The game's sound. Effects are synthesised at start-up (so the game needs
//! no audio files), shaped for a stone cave: bone clatter, slime squelch,
//! dripping water with a long echo, the Heart's beat, blood rain, footsteps
//! and more. A recorded file `data/audio/<name>.wav` or `.ogg` replaces a
//! synthesised effect of the same name. Sounds from the dungeon are placed
//! around the listener (quieter with distance, panned left or right), and
//! two loops — the cave's air and the blood rain — play underneath.
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use velvet_math::Vec3;

use super::dungeon::sound::{Sfx, Sound};
use super::soundtrack::{self, Music, Zone};

#[path = "ambience.rs"]
mod ambience;
pub(crate) use ambience::Amb as ambience_amb;
pub(crate) use ambience::{Emitter, Ground, Scene};

const RATE: u32 = 44_100;
/// A scene's sound, at full slider.
const SCENE_VOLUME: f32 = 0.9;

// ------------------------------------------------------------ synthesis

/// A tiny deterministic noise source.
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn buffer(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f32) as usize]
}

fn t_of(i: usize) -> f32 {
    i as f32 / RATE as f32
}

/// Attack then exponential decay.
fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        t / attack.max(1e-4)
    } else {
        (-(t - attack) / decay.max(1e-4)).exp()
    }
}

/// One-pole low-pass in place.
fn lowpass(signal: &mut [f32], cutoff: f32) {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / RATE as f32).exp();
    let mut y = 0.0;
    for s in signal.iter_mut() {
        y += a * (*s - y);
        *s = y;
    }
}

/// One-pole high-pass in place.
fn highpass(signal: &mut [f32], cutoff: f32) {
    let rc = 1.0 / (std::f32::consts::TAU * cutoff);
    let dt = 1.0 / RATE as f32;
    let a = rc / (rc + dt);
    let (mut prev_x, mut prev_y) = (0.0, 0.0);
    for s in signal.iter_mut() {
        let y = a * (prev_y + *s - prev_x);
        prev_x = *s;
        prev_y = y;
        *s = y;
    }
}

/// A resonant band-pass (biquad) in place.
fn bandpass(signal: &mut [f32], centre: f32, q: f32) {
    let w = std::f32::consts::TAU * centre / RATE as f32;
    let alpha = w.sin() / (2.0 * q);
    let (b0, b2) = (alpha, -alpha);
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * w.cos();
    let a2 = 1.0 - alpha;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    for s in signal.iter_mut() {
        let x = *s;
        let y = (b0 * x + b2 * x2 - a1 * y1 - a2 * y2) / a0;
        x2 = x1;
        x1 = x;
        y2 = y1;
        y1 = y;
        *s = y;
    }
}

/// Deep, soft noise (integrated white noise): rumble without hiss.
fn brown(noise: &mut Noise, n: usize) -> Vec<f32> {
    let mut y = 0.0f32;
    (0..n)
        .map(|_| {
            y = (y + noise.next() * 0.1) * 0.985;
            y * 4.0
        })
        .collect()
}

/// A band-pass whose centre glides over time (a state-variable filter that
/// keeps its state, so the sweep is smooth — no block seams, no crackle).
fn swept_bandpass(signal: &mut [f32], centre: impl Fn(f32) -> f32, q: f32) {
    let (mut low, mut band) = (0.0f32, 0.0f32);
    let damp = 1.0 / q;
    let n = signal.len().max(1);
    for (i, s) in signal.iter_mut().enumerate() {
        let f = 2.0 * (std::f32::consts::PI * centre(i as f32 / n as f32) / RATE as f32).sin();
        low += f * band;
        let high = *s - low - damp * band;
        band += f * high;
        *s = band;
    }
}

/// Add a short burst of band-passed noise at `at` seconds (a click, a tock).
fn click(out: &mut [f32], at: f32, centre: f32, q: f32, length: f32, gain: f32, noise: &mut Noise) {
    let mut burst: Vec<f32> = (0..(length * RATE as f32) as usize)
        .map(|i| noise.next() * env(t_of(i), 0.0008, length * 0.25))
        .collect();
    bandpass(&mut burst, centre, q);
    let start = (at * RATE as f32) as usize;
    for (k, v) in burst.into_iter().enumerate() {
        if let Some(s) = out.get_mut(start + k) {
            *s += v * gain * 6.0;
        }
    }
}

/// Add a decaying tone at `at` seconds, gliding from `f0` to `f1` Hz.
#[allow(clippy::too_many_arguments)]
fn tone(
    out: &mut [f32],
    at: f32,
    f0: f32,
    f1: f32,
    length: f32,
    attack: f32,
    decay: f32,
    gain: f32,
) {
    let start = (at * RATE as f32) as usize;
    let n = (length * RATE as f32) as usize;
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = t_of(i);
        let f = f0 + (f1 - f0) * (t / length);
        phase += std::f32::consts::TAU * f / RATE as f32;
        if let Some(s) = out.get_mut(start + i) {
            *s += phase.sin() * env(t, attack, decay) * gain;
        }
    }
}

/// The cave: a small room's reverb (parallel combs through all-passes),
/// with an optional distinct echo off the far wall. Lengthens the sound.
fn cave(signal: Vec<f32>, wet: f32, tail: f32, echo: f32) -> Vec<f32> {
    let mut out = signal.clone();
    out.resize(signal.len() + (tail * RATE as f32) as usize, 0.0);
    let mut wet_bus = vec![0.0f32; out.len()];
    for (delay_ms, feedback) in [(29.7f32, 0.78f32), (37.1, 0.77), (41.1, 0.75), (43.7, 0.74)] {
        let d = (delay_ms * 0.001 * RATE as f32) as usize;
        let mut line = vec![0.0f32; out.len()];
        for i in 0..out.len() {
            let input = if i < signal.len() { signal[i] } else { 0.0 };
            let back = if i >= d { line[i - d] } else { 0.0 };
            line[i] = input + back * feedback;
            wet_bus[i] += line[i] * 0.25;
        }
    }
    for delay_ms in [5.0f32, 1.7] {
        let d = (delay_ms * 0.001 * RATE as f32) as usize;
        let g = 0.7;
        let mut y = vec![0.0f32; wet_bus.len()];
        for i in 0..wet_bus.len() {
            let x = wet_bus[i];
            let xd = if i >= d { wet_bus[i - d] } else { 0.0 };
            let yd = if i >= d { y[i - d] } else { 0.0 };
            y[i] = -g * x + xd + g * yd;
        }
        wet_bus = y;
    }
    lowpass(&mut wet_bus, 3200.0);
    for (o, w) in out.iter_mut().zip(&wet_bus) {
        *o += w * wet;
    }
    if echo > 0.0 {
        // A slap back off the far wall, and its fainter repeats.
        let d = (0.23 * RATE as f32) as usize;
        let mut echoed = out.clone();
        for i in d..echoed.len() {
            echoed[i] += echoed[i - d] * echo;
        }
        out = echoed;
    }
    out
}

fn normalise(mut signal: Vec<f32>, peak: f32) -> Vec<f32> {
    let max = signal.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    for s in &mut signal {
        *s *= peak / max;
    }
    // Soften the very end so nothing clicks.
    let fade = (0.01 * RATE as f32) as usize;
    let n = signal.len();
    for k in 0..fade.min(n) {
        signal[n - 1 - k] *= k as f32 / fade as f32;
    }
    signal
}

/// The synthesised version of an effect.
fn synthesise(sfx: Sfx) -> Vec<f32> {
    let mut noise = Noise(0x9E37_79B9 ^ (sfx as u32).wrapping_mul(0x85EB_CA6B));
    let signal = match sfx {
        Sfx::Footstep => {
            // A soft, muffled step: a low heel thud and a faint scuff, no hiss.
            let n = (0.16 * RATE as f32) as usize;
            let mut out = brown(&mut noise, n);
            for (i, s) in out.iter_mut().enumerate() {
                *s *= env(t_of(i), 0.004, 0.04) * 0.5;
            }
            lowpass(&mut out, 520.0);
            lowpass(&mut out, 520.0);
            tone(&mut out, 0.0, 88.0, 58.0, 0.12, 0.004, 0.035, 1.0);
            cave(out, 0.07, 0.18, 0.0)
        }
        Sfx::Jump => {
            // A soft push of cloth and breath.
            let n = (0.2 * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            lowpass(&mut out, 2500.0);
            swept_bandpass(&mut out, |u| 350.0 + 400.0 * u, 1.0);
            for (i, s) in out.iter_mut().enumerate() {
                *s *= (i as f32 / n as f32 * std::f32::consts::PI).sin().powi(2);
            }
            out
        }
        Sfx::Land => {
            let mut out = buffer(0.2);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.001, 0.03);
            }
            lowpass(&mut out, 700.0);
            tone(&mut out, 0.0, 90.0, 50.0, 0.15, 0.002, 0.05, 1.0);
            cave(out, 0.25, 0.4, 0.0)
        }
        Sfx::Swing | Sfx::EnemySwing => {
            // A smooth whoosh: soft noise through a band that rises and falls
            // as the blade passes, with no harsh top end.
            let length = if sfx == Sfx::Swing { 0.3 } else { 0.38 };
            let n = (length * RATE as f32) as usize;
            let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
            lowpass(&mut out, 3200.0);
            let (lo, hi) = if sfx == Sfx::Swing {
                (260.0, 1150.0)
            } else {
                (170.0, 720.0)
            };
            swept_bandpass(
                &mut out,
                |u| lo + (hi - lo) * (u * std::f32::consts::PI).sin(),
                1.3,
            );
            for (i, s) in out.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                // Rounded rise and fall, peaking just past the middle.
                *s *= (u.powf(0.8) * std::f32::consts::PI).sin().max(0.0).powi(2);
            }
            lowpass(&mut out, 2400.0);
            cave(out, 0.08, 0.25, 0.0)
        }
        Sfx::PlayerHurt => {
            // A blunt hit and a sharp breath.
            let mut out = buffer(0.35);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.001, 0.05);
            }
            lowpass(&mut out, 900.0);
            tone(&mut out, 0.0, 140.0, 70.0, 0.2, 0.001, 0.06, 1.0);
            let mut breath = buffer(0.35);
            for (i, s) in breath.iter_mut().enumerate() {
                let t = t_of(i);
                *s = noise.next()
                    * if t > 0.03 {
                        env(t - 0.03, 0.02, 0.09)
                    } else {
                        0.0
                    };
            }
            bandpass(&mut breath, 1100.0, 1.2);
            for (o, b) in out.iter_mut().zip(&breath) {
                *o += b * 0.8;
            }
            cave(out, 0.2, 0.4, 0.0)
        }
        Sfx::Drink => {
            // Three gulps.
            let mut out = buffer(0.75);
            for k in 0..3 {
                tone(
                    &mut out,
                    0.05 + k as f32 * 0.21,
                    240.0,
                    150.0,
                    0.14,
                    0.01,
                    0.05,
                    0.9,
                );
                click(
                    &mut out,
                    0.05 + k as f32 * 0.21,
                    420.0,
                    3.0,
                    0.05,
                    0.4,
                    &mut noise,
                );
            }
            lowpass(&mut out, 1600.0);
            out
        }
        Sfx::Respawn => {
            // A reversed bell: a swell that resolves.
            let mut out = buffer(1.4);
            for (f, g) in [(330.0, 0.6), (495.0, 0.4), (660.0, 0.3)] {
                let n = out.len();
                let mut phase = 0.0f32;
                for (i, s) in out.iter_mut().enumerate() {
                    phase += std::f32::consts::TAU * f / RATE as f32;
                    let u = i as f32 / n as f32;
                    *s += phase.sin() * u * u * g;
                }
            }
            cave(out, 0.35, 1.0, 0.25)
        }
        Sfx::HitBone => {
            // A dry tock of bone on blade, two knocks and a low body thud.
            let mut out = buffer(0.3);
            click(&mut out, 0.0, 1900.0, 6.0, 0.04, 1.0, &mut noise);
            click(&mut out, 0.028, 2600.0, 8.0, 0.03, 0.6, &mut noise);
            click(&mut out, 0.061, 1500.0, 6.0, 0.03, 0.4, &mut noise);
            tone(&mut out, 0.0, 160.0, 110.0, 0.09, 0.001, 0.03, 0.5);
            cave(out, 0.25, 0.45, 0.12)
        }
        Sfx::BoneCrack => {
            // A sharp snap and splinters.
            let mut out = buffer(0.4);
            let mut snap: Vec<f32> = (0..(0.014 * RATE as f32) as usize)
                .map(|i| noise.next() * env(t_of(i), 0.0008, 0.004))
                .collect();
            highpass(&mut snap, 900.0);
            lowpass(&mut snap, 3800.0);
            for (o, s) in out.iter_mut().zip(snap) {
                *o += s * 1.4;
            }
            tone(&mut out, 0.0, 190.0, 120.0, 0.08, 0.001, 0.025, 0.6);
            for k in 0..7 {
                let at = 0.012 + k as f32 * 0.018 + noise.next().abs() * 0.02;
                click(
                    &mut out,
                    at,
                    1500.0 + noise.next().abs() * 1300.0,
                    7.0,
                    0.022,
                    0.5 - k as f32 * 0.05,
                    &mut noise,
                );
            }
            lowpass(&mut out, 4200.0);
            cave(out, 0.25, 0.5, 0.15)
        }
        Sfx::BoneRattle => {
            // Loose joints knocking as the skeleton walks.
            let mut out = buffer(0.25);
            for k in 0..5 {
                let at = k as f32 * 0.035 + noise.next().abs() * 0.015;
                click(
                    &mut out,
                    at,
                    1500.0 + noise.next().abs() * 1300.0,
                    7.0,
                    0.022,
                    0.55 - k as f32 * 0.07,
                    &mut noise,
                );
            }
            cave(out, 0.15, 0.3, 0.0)
        }
        Sfx::SkeletonDeath => {
            // The frame gives way: a thump, then bones bouncing and settling.
            let mut out = buffer(1.5);
            tone(&mut out, 0.0, 120.0, 60.0, 0.25, 0.002, 0.08, 0.8);
            let mut at = 0.03;
            let mut gap = 0.09;
            let mut gain = 1.0;
            for k in 0..26 {
                let centre = 1200.0 + (k % 5) as f32 * 450.0 + noise.next().abs() * 700.0;
                click(&mut out, at, centre, 6.0, 0.035, gain, &mut noise);
                if k % 3 == 0 {
                    click(
                        &mut out,
                        at + 0.012,
                        centre * 1.4,
                        8.0,
                        0.02,
                        gain * 0.5,
                        &mut noise,
                    );
                }
                at += gap * (0.6 + noise.next().abs() * 0.6);
                gap *= 0.9;
                gain *= 0.9;
                if at > 1.3 {
                    break;
                }
            }
            cave(out, 0.3, 0.7, 0.2)
        }
        Sfx::HitSlime => {
            // A wet squelch: wobbling low tone in muffled noise.
            let mut out = buffer(0.35);
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                let f = 190.0 - t * 280.0 + (t * 180.0).sin() * 30.0;
                phase += std::f32::consts::TAU * f / RATE as f32;
                *s = phase.sin() * env(t, 0.004, 0.09) * 0.8
                    + noise.next() * env(t, 0.001, 0.05) * 0.6 * (1.0 + (t * 220.0).sin());
            }
            lowpass(&mut out, 1100.0);
            cave(out, 0.2, 0.4, 0.0)
        }
        Sfx::SlimeHop => {
            // A soft boing.
            let mut out = buffer(0.25);
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                let f =
                    150.0 + (t * 40.0).sin() * 60.0 * (-t * 10.0).exp() + 60.0 * (-t * 14.0).exp();
                phase += std::f32::consts::TAU * f / RATE as f32;
                *s = phase.sin() * env(t, 0.005, 0.07);
            }
            lowpass(&mut out, 900.0);
            out
        }
        Sfx::SlimeDeath => {
            // A splat and the bubbles popping after.
            let mut out = buffer(0.9);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.002, 0.08);
            }
            lowpass(&mut out, 800.0);
            tone(&mut out, 0.0, 160.0, 50.0, 0.3, 0.003, 0.1, 0.9);
            for k in 0..7 {
                let at = 0.12 + k as f32 * 0.08 + noise.next().abs() * 0.05;
                let f = 350.0 + noise.next().abs() * 400.0;
                tone(&mut out, at, f, f * 1.6, 0.05, 0.003, 0.015, 0.35);
            }
            cave(out, 0.25, 0.5, 0.1)
        }
        Sfx::CrossbowShot => {
            // Twang of the string and the stock's knock.
            let mut out = buffer(0.5);
            click(&mut out, 0.0, 900.0, 3.0, 0.03, 0.8, &mut noise);
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                phase += std::f32::consts::TAU * (150.0 + 40.0 * (-t * 20.0).exp()) / RATE as f32;
                // A buzzing (sawtooth) string.
                let saw = (phase / std::f32::consts::TAU).fract() * 2.0 - 1.0;
                *s += saw * env(t, 0.001, 0.09) * 0.6;
            }
            lowpass(&mut out, 2400.0);
            cave(out, 0.22, 0.4, 0.1)
        }
        Sfx::BoltImpact => {
            let mut out = buffer(0.25);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.0008, 0.02);
            }
            lowpass(&mut out, 1800.0);
            tone(&mut out, 0.0, 420.0, 300.0, 0.08, 0.001, 0.02, 0.6);
            cave(out, 0.25, 0.4, 0.1)
        }
        Sfx::SpellCharge => {
            // Rising shimmer as the orb gathers.
            let mut out = buffer(1.0);
            for (k, f) in [300.0f32, 451.0, 603.0].into_iter().enumerate() {
                let n = out.len();
                let mut phase = 0.0f32;
                for (i, s) in out.iter_mut().enumerate() {
                    let u = i as f32 / n as f32;
                    phase += std::f32::consts::TAU * f * (1.0 + u * 1.2) / RATE as f32;
                    let tremolo = 0.6 + 0.4 * (u * 40.0 + k as f32).sin();
                    *s += phase.sin() * u * tremolo * 0.4;
                }
            }
            cave(out, 0.3, 0.6, 0.15)
        }
        Sfx::SpellBurst => {
            // The orb bursts: a boom, crackle and sparkle.
            let mut out = buffer(1.0);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.002, 0.18);
            }
            lowpass(&mut out, 900.0);
            tone(&mut out, 0.0, 90.0, 40.0, 0.6, 0.003, 0.2, 1.0);
            for k in 0..8 {
                click(
                    &mut out,
                    0.05 + k as f32 * 0.05,
                    3000.0 + k as f32 * 200.0,
                    10.0,
                    0.02,
                    0.3,
                    &mut noise,
                );
            }
            cave(out, 0.35, 0.9, 0.25)
        }
        Sfx::Shockwave => {
            // A deep boom rolling out through the stone.
            let mut out = buffer(1.6);
            tone(&mut out, 0.0, 60.0, 28.0, 1.4, 0.005, 0.45, 1.0);
            for (i, s) in out.iter_mut().enumerate() {
                *s += noise.next() * env(t_of(i), 0.004, 0.35) * 0.7;
            }
            lowpass(&mut out, 420.0);
            cave(out, 0.4, 1.2, 0.3)
        }
        Sfx::Roar => {
            // A throaty growl swelling and breaking.
            let mut out = buffer(1.6);
            let n = out.len();
            let mut phase = 0.0f32;
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                let u = i as f32 / n as f32;
                let f = 78.0 + (t * 7.0).sin() * 6.0 + u * 20.0;
                phase += std::f32::consts::TAU * f / RATE as f32;
                let saw = (phase / std::f32::consts::TAU).fract() * 2.0 - 1.0;
                let shape = (u * std::f32::consts::PI).sin().powf(0.6);
                *s = (saw * 0.7 + noise.next() * 0.5) * shape;
            }
            bandpass(&mut out, 520.0, 0.8);
            cave(out, 0.4, 1.3, 0.35)
        }
        Sfx::Summon => {
            // A dark swell and a low chord from below.
            let mut out = buffer(1.8);
            let n = out.len();
            for (f, g) in [(55.0f32, 0.7), (82.4, 0.5), (116.5, 0.35)] {
                let mut phase = 0.0f32;
                for (i, s) in out.iter_mut().enumerate() {
                    let u = i as f32 / n as f32;
                    phase += std::f32::consts::TAU * f / RATE as f32;
                    *s += phase.sin() * (u * std::f32::consts::PI).sin() * g;
                }
            }
            let mut hiss = buffer(1.8);
            for (i, s) in hiss.iter_mut().enumerate() {
                let u = i as f32 / n as f32;
                *s = noise.next() * u * u;
            }
            bandpass(&mut hiss, 700.0, 0.8);
            for (o, h) in out.iter_mut().zip(&hiss) {
                *o += h * 0.5;
            }
            cave(out, 0.4, 1.2, 0.3)
        }
        Sfx::Rubble => {
            // Stone breaking and pieces tumbling down.
            let mut out = buffer(1.1);
            for (i, s) in out.iter_mut().enumerate() {
                *s = noise.next() * env(t_of(i), 0.002, 0.12);
            }
            lowpass(&mut out, 1200.0);
            let mut at = 0.05;
            for k in 0..18 {
                click(
                    &mut out,
                    at,
                    700.0 + noise.next().abs() * 1500.0,
                    3.0,
                    0.04,
                    0.8 - k as f32 * 0.035,
                    &mut noise,
                );
                at += 0.03 + noise.next().abs() * 0.05;
            }
            cave(out, 0.3, 0.8, 0.2)
        }
        Sfx::HeartBeat => {
            // Lub... dub: two deep, muffled thumps.
            let mut out = buffer(0.6);
            tone(&mut out, 0.0, 62.0, 42.0, 0.2, 0.012, 0.06, 1.0);
            tone(&mut out, 0.0, 124.0, 84.0, 0.1, 0.008, 0.03, 0.35);
            tone(&mut out, 0.26, 54.0, 38.0, 0.22, 0.012, 0.07, 0.8);
            lowpass(&mut out, 240.0);
            cave(out, 0.35, 0.8, 0.2)
        }
        Sfx::ChestCreak => {
            // An old lid's creak, then the latch dropping back.
            let mut out = buffer(0.95);
            let mut phase = 0.0f32;
            let n = out.len();
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                let u = i as f32 / n as f32;
                let f = 95.0 + u * 55.0 + (t * 23.0).sin() * 18.0 + noise.next() * 6.0;
                phase += std::f32::consts::TAU * f / RATE as f32;
                let saw = (phase / std::f32::consts::TAU).fract() * 2.0 - 1.0;
                let grain = if (t * 38.0).fract() < 0.35 { 1.0 } else { 0.45 };
                *s = saw
                    * grain
                    * (u * std::f32::consts::PI).sin().powf(0.5)
                    * if t < 0.8 { 1.0 } else { 0.0 };
            }
            bandpass(&mut out, 1100.0, 1.5);
            click(&mut out, 0.82, 1300.0, 4.0, 0.05, 1.2, &mut noise);
            cave(out, 0.25, 0.5, 0.1)
        }
        Sfx::Coins => {
            // Coins and trinkets chiming as they spill.
            let mut out = buffer(1.0);
            let mut at = 0.0;
            for k in 0..10 {
                let f = 1250.0 + noise.next().abs() * 1300.0;
                tone(&mut out, at, f, f, 0.25, 0.002, 0.07, 0.4);
                tone(&mut out, at, f * 2.1, f * 2.1, 0.12, 0.002, 0.03, 0.08);
                at += 0.035 + noise.next().abs() * 0.07 + k as f32 * 0.004;
            }
            cave(out, 0.25, 0.5, 0.0)
        }
        Sfx::Pickup => {
            let mut out = buffer(0.4);
            tone(&mut out, 0.0, 880.0, 880.0, 0.2, 0.002, 0.07, 0.6);
            tone(&mut out, 0.08, 1320.0, 1320.0, 0.25, 0.002, 0.09, 0.5);
            cave(out, 0.2, 0.4, 0.0)
        }
        Sfx::Relic => {
            // A bright rising arpeggio.
            let mut out = buffer(1.6);
            for (k, f) in [523.3f32, 659.3, 784.0, 1046.5].into_iter().enumerate() {
                tone(&mut out, k as f32 * 0.12, f, f, 1.0, 0.004, 0.35, 0.5);
                tone(
                    &mut out,
                    k as f32 * 0.12,
                    f * 2.0,
                    f * 2.0,
                    0.6,
                    0.004,
                    0.18,
                    0.15,
                );
            }
            cave(out, 0.3, 1.0, 0.2)
        }
        Sfx::RuneChime => {
            // A stone bell: inharmonic partials ringing out.
            let mut out = buffer(1.6);
            for (f, g, d) in [
                (660.0f32, 0.6, 0.6),
                (1789.0, 0.3, 0.3),
                (2437.0, 0.2, 0.2),
                (3301.0, 0.1, 0.12),
            ] {
                tone(&mut out, 0.0, f, f, 1.5, 0.002, d, g);
            }
            cave(out, 0.35, 1.2, 0.25)
        }
        Sfx::SealOpen => {
            // The seal wakes: stone grinding under a rising chord.
            let mut out = buffer(2.4);
            for (i, s) in out.iter_mut().enumerate() {
                let t = t_of(i);
                *s = noise.next() * (t * std::f32::consts::PI / 2.4).sin() * 0.6;
            }
            lowpass(&mut out, 300.0);
            for (k, f) in [220.0f32, 277.2, 329.6, 440.0].into_iter().enumerate() {
                tone(&mut out, 0.3 + k as f32 * 0.25, f, f, 1.8, 0.3, 0.8, 0.35);
            }
            cave(out, 0.4, 1.5, 0.3)
        }
        Sfx::StepGrass | Sfx::StepCobble | Sfx::StepHall | Sfx::StepWood | Sfx::StepWater => {
            ambience::step(sfx, &mut noise)
        }
        Sfx::Drip => {
            // A drop meeting still water, and the cave answering.
            let mut out = buffer(0.12);
            tone(&mut out, 0.0, 700.0, 1700.0, 0.05, 0.001, 0.012, 1.0);
            tone(&mut out, 0.018, 1200.0, 2100.0, 0.04, 0.001, 0.01, 0.4);
            cave(out, 0.6, 1.6, 0.4)
        }
    };
    normalise(signal, 0.9)
}

/// The cave's air: a low drone with a faint moving draught (a seamless loop).
fn cave_air() -> Vec<f32> {
    let seconds = 8.0;
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(0xA11C_E5E5);
    let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    lowpass(&mut out, 180.0);
    lowpass(&mut out, 180.0);
    for (i, s) in out.iter_mut().enumerate() {
        let t = t_of(i);
        // Whole cycles per loop keep the seam silent.
        let drone = (t * std::f32::consts::TAU * 55.0).sin() * 0.25
            + (t * std::f32::consts::TAU * 82.5).sin() * 0.12
            + (t * std::f32::consts::TAU * 0.25).sin() * 0.1;
        let draught = 0.6 + 0.4 * (t * std::f32::consts::TAU * 0.125).sin();
        *s = *s * 3.0 * draught + drone * 0.5;
    }
    crossfade_loop(normalise(out, 0.5))
}

/// Blood rain: dense patter with heavier drops (a seamless loop).
fn rain() -> Vec<f32> {
    let seconds = 4.0;
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(0xB100_D5ED);
    let mut out: Vec<f32> = (0..n).map(|_| noise.next()).collect();
    highpass(&mut out, 450.0);
    lowpass(&mut out, 2600.0);
    lowpass(&mut out, 2600.0);
    for s in &mut out {
        *s *= 0.5;
    }
    for _ in 0..110 {
        let at = noise.next().abs() * (seconds - 0.05);
        click(
            &mut out,
            at,
            700.0 + noise.next().abs() * 1100.0,
            3.0,
            0.03,
            0.3,
            &mut noise,
        );
    }
    lowpass(&mut out, 3000.0);
    crossfade_loop(normalise(out, 0.5))
}

/// Blend a loop's end into its start so it repeats without a seam.
fn crossfade_loop(mut signal: Vec<f32>) -> Vec<f32> {
    let fade = (0.5 * RATE as f32) as usize;
    let n = signal.len();
    if n <= fade * 2 {
        return signal;
    }
    for k in 0..fade {
        let a = k as f32 / fade as f32;
        signal[k] = signal[k] * a + signal[n - fade + k] * (1.0 - a);
    }
    signal.truncate(n - fade);
    signal
}

// ------------------------------------------------------------ playback

/// A sound ready to play: mono samples at `rate`, or a recorded file.
struct Clip {
    samples: Vec<f32>,
    channels: u16,
    rate: u32,
}

/// Load `data/audio/<name>.wav|ogg` if the project has a recording of it.
fn recorded(name: &str) -> Option<Clip> {
    use std::io::BufReader;
    let root = crate::exploration3d_state::data_dir().join("audio");
    for ext in ["wav", "ogg"] {
        let path = root.join(format!("{name}.{ext}"));
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        match rodio::Decoder::new(BufReader::new(file)) {
            Ok(decoder) => {
                let (channels, rate) = (decoder.channels(), decoder.sample_rate());
                return Some(Clip {
                    samples: decoder.collect(),
                    channels,
                    rate,
                });
            }
            Err(error) => eprintln!("[audio] {}: {error}", path.display()),
        }
    }
    None
}

fn muted() -> bool {
    std::env::var_os("VELVET_MUTE").is_some()
}

/// Every sound the game makes, built (or read from `data/audio`) and waiting
/// for an output. Plain data, so it can be made on a background thread.
pub(crate) struct Synth {
    bank: BTreeMap<Sfx, Clip>,
    air: Vec<f32>,
    rain: Vec<f32>,
    ambience: ambience::Data,
}

impl Synth {
    pub(crate) fn build() -> Self {
        let started = Instant::now();
        let bank = Sfx::ALL
            .into_iter()
            .map(|sfx| {
                let clip = recorded(sfx.name()).unwrap_or_else(|| Clip {
                    samples: synthesise(sfx),
                    channels: 1,
                    rate: RATE,
                });
                (sfx, clip)
            })
            .collect();
        let synth = Self {
            bank,
            air: cave_air(),
            rain: rain(),
            ambience: ambience::Data::build(),
        };
        eprintln!(
            "[audio] {} effects and the ambience made in {:.0} ms",
            Sfx::ALL.len(),
            started.elapsed().as_secs_f32() * 1000.0
        );
        synth
    }
}

/// The game's audio. `None`-safe: without a sound device the game is silent.
pub(crate) struct GameAudio {
    stream: OutputStream,
    bank: BTreeMap<Sfx, Clip>,
    air: Sink,
    rain: Sink,
    /// When each effect last started, to thin out repeats in one instant.
    last: BTreeMap<Sfx, Instant>,
    drip_in: f32,
    noise: Noise,
    /// The soundtrack, if the project ships one.
    music: Option<Music>,
    /// Wind, birds, torches: the living background of each place.
    ambience: ambience::Ambience,
    /// How loud the effects of the last moments were (0..1): the music ducks to it.
    effect_level: f32,
    /// A scene's own sound (the bed scene's clips), over a hushed soundtrack.
    scene: Option<Sink>,
}

impl GameAudio {
    /// Open the default output and prepare every effect. `None` when there is
    /// no sound device or `VELVET_MUTE` is set.
    pub(crate) fn new() -> Option<Self> {
        if muted() {
            return None;
        }
        Self::from_synth(Synth::build())
    }

    /// Opens the output and starts the sinks around sounds already made,
    /// which is quick; `Synth::build` is the slow part and can run on
    /// another thread.
    pub(crate) fn from_synth(synth: Synth) -> Option<Self> {
        if muted() {
            return None;
        }
        let mut stream = match OutputStreamBuilder::open_default_stream() {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("[audio] no sound output ({error}); the game stays silent");
                return None;
            }
        };
        stream.log_on_drop(false);
        let started = Instant::now();
        let air = Sink::connect_new(stream.mixer());
        air.append(SamplesBuffer::new(1, RATE, synth.air).repeat_infinite());
        air.set_volume(0.0);
        let rain_sink = Sink::connect_new(stream.mixer());
        rain_sink.append(SamplesBuffer::new(1, RATE, synth.rain).repeat_infinite());
        rain_sink.set_volume(0.0);
        let ambience = ambience::Ambience::new(&stream, synth.ambience);
        let music = Music::new(stream.mixer().clone(), &soundtrack::music_dir());
        eprintln!(
            "[audio] output, music and ambience opened in {:.0} ms",
            started.elapsed().as_secs_f32() * 1000.0
        );
        Some(Self {
            stream,
            bank: synth.bank,
            air,
            rain: rain_sink,
            last: BTreeMap::new(),
            drip_in: 2.0,
            noise: Noise(0xD1CE),
            music,
            ambience,
            effect_level: 0.0,
            scene: None,
        })
    }

    /// Play one dungeon sound as heard from `listener` facing `forward`.
    pub(crate) fn play(&mut self, sound: &Sound, listener: Vec3, forward: Vec3) {
        let now = Instant::now();
        if self
            .last
            .get(&sound.sfx)
            .is_some_and(|t| now.duration_since(*t) < Duration::from_millis(45))
        {
            return;
        }
        let Some(clip) = self.bank.get(&sound.sfx) else {
            return;
        };
        // Distance falls off gently (the cave carries sound); sources to one
        // side lean into that ear.
        let (gain, pan) = match sound.at {
            Some(at) => {
                let to = at - listener;
                let flat = Vec3::new(to.x, 0.0, to.z);
                let distance = to.length();
                let gain = 1.0 / (1.0 + (distance / 5.0).powf(1.4));
                if gain * sound.volume < 0.02 {
                    return;
                }
                let right = Vec3::new(-forward.z, 0.0, forward.x).normalize_or_zero();
                let pan = if flat.length() > 0.3 {
                    flat.normalize_or_zero().dot(right).clamp(-1.0, 1.0) * 0.75
                } else {
                    0.0
                };
                (gain, pan)
            }
            None => (1.0, 0.0),
        };
        self.last.insert(sound.sfx, now);
        let volume = (sound.volume * gain * 0.8).min(1.0);
        // Anything that is not a step or a drip pushes the music down a little.
        if !matches!(
            sound.sfx,
            Sfx::Footstep
                | Sfx::StepGrass
                | Sfx::StepCobble
                | Sfx::StepHall
                | Sfx::StepWood
                | Sfx::StepWater
                | Sfx::Drip
        ) {
            self.effect_level = self.effect_level.max((volume * 1.6).min(1.0));
        }
        let sink = Sink::connect_new(self.stream.mixer());
        if clip.channels == 1 {
            let (left, right) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
            // A little pitch life, so repeats don't sound stamped.
            let speed = 0.94 + (self.noise.next() * 0.5 + 0.5) * 0.12;
            let mut stereo = Vec::with_capacity(clip.samples.len() * 2);
            for s in &clip.samples {
                stereo.push(s * left);
                stereo.push(s * right);
            }
            sink.append(SamplesBuffer::new(2, clip.rate, stereo).speed(
                if sound.sfx == Sfx::HeartBeat {
                    1.0
                } else {
                    speed
                },
            ));
        } else {
            sink.append(SamplesBuffer::new(
                clip.channels,
                clip.rate,
                clip.samples.clone(),
            ));
        }
        sink.set_volume(volume * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Effects));
        sink.detach();
    }

    /// Each frame: the soundtrack and the place's ambience follow the scene,
    /// the cave's air and the rain fade with it, and the effects keep
    /// the music ducked while they sound.
    pub(crate) fn update_scene(&mut self, scene: Scene, rain: f32, dt: f32) {
        self.effect_level = (self.effect_level - dt * 1.4).max(0.0);
        if let Some(music) = &mut self.music {
            music.update(scene.zone, dt, self.effect_level, scene.paused);
        }
        self.ambience.update(self.stream.mixer(), &scene, dt);
        // A scene's sound follows its slider as it is moved.
        if let Some(sink) = &self.scene {
            sink.set_volume(SCENE_VOLUME * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Scenes));
        }
        self.update_cave(
            scene.zone == Zone::Cave,
            rain,
            dt,
            scene.listener,
            scene.forward,
        );
    }

    /// The hush while the pause menu is open; the next scene update lifts it.
    /// Plays a scene's sound file (`None` stops it), looped or once; the soundtrack and the
    /// place's ambience hush while one plays.
    pub(crate) fn scene_sound(&mut self, path: Option<&std::path::Path>, looped: bool) {
        use std::io::BufReader;
        if let Some(sink) = self.scene.take() {
            sink.stop();
        }
        self.set_paused(path.is_some());
        let Some(path) = path else {
            return;
        };
        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        let decoder = match rodio::Decoder::new(BufReader::new(file)) {
            Ok(decoder) => decoder,
            Err(error) => {
                eprintln!("[audio] {}: {error}", path.display());
                return;
            }
        };
        let (channels, rate) = (decoder.channels(), decoder.sample_rate());
        let samples: Vec<f32> = decoder.collect();
        let sink = Sink::connect_new(self.stream.mixer());
        let buffer = SamplesBuffer::new(channels, rate, samples);
        if looped {
            sink.append(buffer.repeat_infinite());
        } else {
            sink.append(buffer);
        }
        sink.set_volume(SCENE_VOLUME * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Scenes));
        self.scene = Some(sink);
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
        if let Some(music) = &mut self.music {
            music.hush(paused);
        }
        self.ambience.hush(paused);
    }

    /// The cave's air and the rain follow the scene, and water drips here and
    /// there around the listener while underground.
    fn update_cave(
        &mut self,
        underground: bool,
        rain: f32,
        dt: f32,
        listener: Vec3,
        forward: Vec3,
    ) {
        let ambience = crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Ambience);
        let air = if underground { 0.32 * ambience } else { 0.0 };
        let current = self.air.volume();
        self.air
            .set_volume(current + (air - current) * (dt * 1.5).min(1.0));
        self.rain.set_volume(if underground {
            rain.clamp(0.0, 1.0) * 0.55 * ambience
        } else {
            0.0
        });
        if !underground {
            return;
        }
        self.drip_in -= dt;
        if self.drip_in <= 0.0 {
            self.drip_in = 1.6 + (self.noise.next() * 0.5 + 0.5) * 4.5;
            let angle = self.noise.next() * std::f32::consts::PI;
            let reach = 2.5 + (self.noise.next() * 0.5 + 0.5) * 7.0;
            let at = listener + Vec3::new(angle.cos() * reach, 1.5, angle.sin() * reach);
            let volume = 0.35 + (self.noise.next() * 0.5 + 0.5) * 0.3;
            self.play(
                &Sound {
                    sfx: Sfx::Drip,
                    at: Some(at),
                    volume,
                },
                listener,
                forward,
            );
        }
    }
}

/// Write every synthesised effect and loop as a 16-bit WAV into `dir`, to
/// listen to them, or to edit one and drop it in `data/audio/`.
pub(crate) fn export(dir: &std::path::Path) -> std::io::Result<usize> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let mut written = 0;
    let clips = Sfx::ALL
        .into_iter()
        .map(|sfx| (sfx.name().to_string(), synthesise(sfx)))
        .chain([
            ("cave_air".to_string(), cave_air()),
            ("blood_rain".to_string(), rain()),
        ])
        .chain(ambience::export_clips());
    for (name, samples) in clips {
        let mut file = std::fs::File::create(dir.join(format!("{name}.wav")))?;
        let data_len = (samples.len() * 2) as u32;
        file.write_all(b"RIFF")?;
        file.write_all(&(36 + data_len).to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&RATE.to_le_bytes())?;
        file.write_all(&(RATE * 2).to_le_bytes())?;
        file.write_all(&2u16.to_le_bytes())?;
        file.write_all(&16u16.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&data_len.to_le_bytes())?;
        let pcm: Vec<u8> = samples
            .iter()
            .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
            .collect();
        file.write_all(&pcm)?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays a second of every zone through the real output: run by hand with
    /// `cargo test -p demon-queen game_audio -- --ignored`.
    #[test]
    #[ignore = "opens the sound device and is audible"]
    fn every_zone_starts_its_music_and_its_ambience() {
        let Some(mut audio) = GameAudio::new() else {
            return;
        };
        for zone in Zone::ALL {
            for _ in 0..24 {
                let scene = Scene {
                    emitters: &[],
                    zone,
                    night: 0.6,
                    listener: Vec3::ZERO,
                    forward: Vec3::Z,
                    paused: false,
                    springs: 0.0,
                };
                audio.update_scene(scene, 0.0, 0.05);
                std::thread::sleep(Duration::from_millis(50));
            }
            if let Some(music) = &audio.music {
                assert!(music.playing() >= 1, "{zone:?} has music playing");
            }
        }
        audio.play(
            &Sound {
                sfx: Sfx::StepGrass,
                at: None,
                volume: 0.4,
            },
            Vec3::ZERO,
            Vec3::Z,
        );
        audio.set_paused(true);
        audio.set_paused(false);
    }

    #[test]
    fn every_effect_synthesises_clean_audible_sound() {
        for sfx in Sfx::ALL {
            let clip = synthesise(sfx);
            assert!(clip.len() > 1000, "{sfx:?} has length");
            assert!(
                clip.iter().all(|s| s.is_finite() && s.abs() <= 1.0),
                "{sfx:?} stays in range"
            );
            let peak = clip.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.5, "{sfx:?} is audible");
        }
        for (name, loop_) in [("air", cave_air()), ("rain", rain())] {
            assert!(loop_.iter().all(|s| s.is_finite()), "{name} finite");
            // The seam: end and start meet without a jump.
            assert!(
                (loop_[0] - loop_[loop_.len() - 1]).abs() < 0.25,
                "{name} loops smoothly"
            );
        }
    }
}
