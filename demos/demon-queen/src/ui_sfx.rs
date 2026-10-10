//! The menus' own sounds: soft, short and quiet, made in code like the game's other effects.
//! Every menu (the pause menu and its pages, the shop, the stalls) calls `play`; the sounds
//! open their own output the first time one is needed, so they work in the visual novel, in
//! the 3D world and in the tools alike, and stay silent where there is no device or with
//! `VELVET_MUTE` set.
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamBuilder, Sink};

const RATE: u32 = 44_100;

/// A menu sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum UiSfx {
    /// The focus moves to another entry.
    Move,
    /// An entry is chosen.
    Select,
    /// Back, cancel, close a pane.
    Back,
    /// A menu opens.
    Open,
    /// A menu closes.
    Close,
    /// Another page or tab.
    Page,
    /// Not allowed.
    Error,
    /// An item taken in hand.
    Pick,
    /// An item put down.
    Drop,
    /// An item turned.
    Rotate,
    /// Gear put on.
    Equip,
    /// Money changes hands.
    Coins,
    /// A deal struck: the haggle won.
    Deal,
    /// A haggle lost, a fusion failed.
    Fail,
    /// Two things become one.
    Fuse,
    /// A small tick (a needle passing a mark, a counter).
    Tick,
}

impl UiSfx {
    const ALL: [UiSfx; 16] = [
        UiSfx::Move,
        UiSfx::Select,
        UiSfx::Back,
        UiSfx::Open,
        UiSfx::Close,
        UiSfx::Page,
        UiSfx::Error,
        UiSfx::Pick,
        UiSfx::Drop,
        UiSfx::Rotate,
        UiSfx::Equip,
        UiSfx::Coins,
        UiSfx::Deal,
        UiSfx::Fail,
        UiSfx::Fuse,
        UiSfx::Tick,
    ];

    /// How loud it plays (the menus stay under the music).
    fn volume(self) -> f32 {
        match self {
            UiSfx::Move | UiSfx::Tick => 0.22,
            UiSfx::Page | UiSfx::Rotate => 0.30,
            UiSfx::Open | UiSfx::Close => 0.34,
            UiSfx::Deal | UiSfx::Fuse => 0.46,
            _ => 0.38,
        }
    }

    /// The least time between two of the same (a held key does not machine-gun).
    fn spacing(self) -> Duration {
        Duration::from_millis(match self {
            UiSfx::Move | UiSfx::Tick => 28,
            _ => 60,
        })
    }
}

struct UiAudio {
    /// Never closed: it lives as long as the program, and at its end Windows has already stopped
    /// the stream's thread, so closing it then waits on a thread that is gone (and aborts).
    stream: std::mem::ManuallyDrop<OutputStream>,
    bank: HashMap<UiSfx, Vec<f32>>,
    last: HashMap<UiSfx, Instant>,
}

thread_local! {
    /// Opened on the first sound; `Some(None)` once it is known there is no output.
    static AUDIO: RefCell<Option<Option<UiAudio>>> = const { RefCell::new(None) };
}

fn muted() -> bool {
    std::env::var_os("VELVET_MUTE").is_some()
}

/// Plays a menu sound (nothing when muted or without a sound device).
pub(crate) fn play(sfx: UiSfx) {
    // Unit tests walk the menus too; they stay silent.
    if cfg!(test) || muted() {
        return;
    }
    AUDIO.with(|cell| {
        let mut slot = cell.borrow_mut();
        let audio = slot.get_or_insert_with(open);
        let Some(audio) = audio.as_mut() else {
            return;
        };
        let now = Instant::now();
        if audio
            .last
            .get(&sfx)
            .is_some_and(|t| now.duration_since(*t) < sfx.spacing())
        {
            return;
        }
        audio.last.insert(sfx, now);
        let Some(samples) = audio.bank.get(&sfx) else {
            return;
        };
        let sink = Sink::connect_new(audio.stream.mixer());
        sink.append(SamplesBuffer::new(1, RATE, samples.clone()));
        sink.set_volume(sfx.volume());
        sink.detach();
    });
}

fn open() -> Option<UiAudio> {
    let mut stream = match OutputStreamBuilder::open_default_stream() {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("[ui-sfx] no sound output ({error}); the menus stay silent");
            return None;
        }
    };
    stream.log_on_drop(false);
    let bank = UiSfx::ALL.iter().map(|&s| (s, synthesise(s))).collect();
    Some(UiAudio {
        stream: std::mem::ManuallyDrop::new(stream),
        bank,
        last: HashMap::new(),
    })
}

// ------------------------------------------------------------------ synthesis

fn buffer(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f32) as usize]
}

fn t_of(i: usize) -> f32 {
    i as f32 / RATE as f32
}

/// A soft attack then an exponential fall.
fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        (t / attack.max(1e-4)).powf(0.7)
    } else {
        (-(t - attack) / decay.max(1e-4)).exp()
    }
}

/// A bell-like note at `at` seconds: a sine with a quieter octave and a little shimmer.
fn note(out: &mut [f32], at: f32, freq: f32, length: f32, gain: f32) {
    let start = (at * RATE as f32) as usize;
    for i in start..out.len() {
        let t = t_of(i - start);
        if t > length * 4.0 {
            break;
        }
        let e = env(t, 0.004, length);
        let p = TAU * freq * t;
        out[i] += gain
            * e
            * (p.sin()
                + 0.28 * (2.0 * p).sin() * (-t / (length * 0.5)).exp()
                + 0.08 * (3.01 * p).sin());
    }
}

/// A gliding tone from `f0` to `f1`.
fn glide(out: &mut [f32], at: f32, f0: f32, f1: f32, length: f32, gain: f32) {
    let start = (at * RATE as f32) as usize;
    let mut phase = 0.0f32;
    for i in start..out.len() {
        let t = t_of(i - start);
        if t > length {
            break;
        }
        let k = t / length;
        let f = f0 + (f1 - f0) * k;
        phase += TAU * f / RATE as f32;
        let e = (1.0 - k).powf(1.6) * (t / 0.006).min(1.0);
        out[i] += gain * e * phase.sin();
    }
}

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Noise through a band-pass swept from `c0` to `c1` Hz, under an envelope: air, paper, cloth.
fn swish(out: &mut [f32], at: f32, c0: f32, c1: f32, length: f32, gain: f32, seed: u32) {
    let start = (at * RATE as f32) as usize;
    let mut noise = Noise(seed);
    let (mut low, mut band) = (0.0f32, 0.0f32);
    for i in start..out.len() {
        let t = t_of(i - start);
        if t > length {
            break;
        }
        let k = t / length;
        let c = c0 + (c1 - c0) * k;
        // A state-variable filter, q about 1.6.
        let f = 2.0 * (std::f32::consts::PI * c / RATE as f32).sin();
        let high = noise.next() - low - 0.62 * band;
        band += f * high;
        low += f * band;
        let e = (k * 4.0).min(1.0) * (1.0 - k).powf(1.3);
        out[i] += gain * e * band;
    }
}

/// A short click: a burst of filtered noise.
fn click(out: &mut [f32], at: f32, centre: f32, length: f32, gain: f32, seed: u32) {
    swish(out, at, centre, centre * 0.8, length, gain, seed);
}

/// A soft low thump.
fn thump(out: &mut [f32], at: f32, freq: f32, length: f32, gain: f32) {
    glide(out, at, freq * 1.6, freq, length, gain);
}

fn normalise(mut s: Vec<f32>, peak: f32) -> Vec<f32> {
    let most = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if most > 1e-6 {
        for v in &mut s {
            *v *= peak / most;
        }
    }
    // A few milliseconds of fade at the end, so nothing clicks off.
    let n = s.len();
    let fade = (RATE as usize / 200).min(n);
    for i in 0..fade {
        s[n - 1 - i] *= i as f32 / fade as f32;
    }
    s
}

fn synthesise(sfx: UiSfx) -> Vec<f32> {
    let mut s;
    match sfx {
        UiSfx::Move => {
            s = buffer(0.07);
            note(&mut s, 0.0, 1975.5, 0.018, 0.7);
            click(&mut s, 0.0, 5200.0, 0.012, 0.25, 11);
        }
        UiSfx::Tick => {
            s = buffer(0.05);
            click(&mut s, 0.0, 3800.0, 0.014, 0.8, 5);
        }
        UiSfx::Select => {
            s = buffer(0.42);
            note(&mut s, 0.0, 1318.5, 0.07, 0.6);
            note(&mut s, 0.055, 1975.5, 0.12, 0.55);
            swish(&mut s, 0.0, 3000.0, 6000.0, 0.06, 0.15, 7);
        }
        UiSfx::Back => {
            s = buffer(0.32);
            note(&mut s, 0.0, 1174.7, 0.05, 0.5);
            note(&mut s, 0.05, 880.0, 0.09, 0.45);
        }
        UiSfx::Open => {
            s = buffer(0.5);
            swish(&mut s, 0.0, 700.0, 3600.0, 0.24, 0.9, 21);
            note(&mut s, 0.14, 1568.0, 0.11, 0.35);
            note(&mut s, 0.19, 2349.3, 0.16, 0.30);
        }
        UiSfx::Close => {
            s = buffer(0.36);
            swish(&mut s, 0.0, 3000.0, 600.0, 0.22, 0.85, 23);
            note(&mut s, 0.06, 1174.7, 0.08, 0.22);
        }
        UiSfx::Page => {
            s = buffer(0.2);
            swish(&mut s, 0.0, 2400.0, 5200.0, 0.11, 0.9, 31);
            click(&mut s, 0.09, 4200.0, 0.01, 0.2, 33);
        }
        UiSfx::Error => {
            s = buffer(0.3);
            thump(&mut s, 0.0, 170.0, 0.08, 0.8);
            thump(&mut s, 0.1, 150.0, 0.1, 0.7);
        }
        UiSfx::Pick => {
            s = buffer(0.16);
            note(&mut s, 0.0, 987.8, 0.03, 0.6);
            click(&mut s, 0.0, 2600.0, 0.02, 0.3, 41);
        }
        UiSfx::Drop => {
            s = buffer(0.2);
            thump(&mut s, 0.0, 230.0, 0.06, 0.8);
            click(&mut s, 0.0, 1400.0, 0.03, 0.35, 43);
        }
        UiSfx::Rotate => {
            s = buffer(0.12);
            click(&mut s, 0.0, 3200.0, 0.012, 0.7, 51);
            click(&mut s, 0.045, 2600.0, 0.014, 0.6, 53);
        }
        UiSfx::Equip => {
            s = buffer(0.5);
            for (f, g) in [(2093.0, 0.45), (3302.0, 0.3), (4710.0, 0.2)] {
                note(&mut s, 0.0, f, 0.14, g);
            }
            click(&mut s, 0.0, 2000.0, 0.02, 0.4, 61);
        }
        UiSfx::Coins => {
            s = buffer(0.5);
            let mut noise = Noise(71);
            for k in 0..4 {
                let at = k as f32 * 0.055 + noise.next().abs() * 0.015;
                note(&mut s, at, 2637.0 + noise.next() * 300.0, 0.05, 0.35);
                note(&mut s, at, 3951.1 + noise.next() * 300.0, 0.04, 0.2);
            }
        }
        UiSfx::Deal => {
            s = buffer(0.8);
            thump(&mut s, 0.0, 120.0, 0.09, 0.9);
            click(&mut s, 0.0, 900.0, 0.04, 0.6, 81);
            for (k, f) in [1046.5f32, 1318.5, 1568.0, 2093.0].iter().enumerate() {
                note(&mut s, 0.07 + k as f32 * 0.06, *f, 0.16, 0.42);
            }
        }
        UiSfx::Fail => {
            s = buffer(0.55);
            glide(&mut s, 0.0, 520.0, 240.0, 0.36, 0.6);
            glide(&mut s, 0.0, 780.0, 360.0, 0.36, 0.25);
        }
        UiSfx::Fuse => {
            s = buffer(1.1);
            swish(&mut s, 0.0, 500.0, 5200.0, 0.55, 0.6, 91);
            for (k, f) in [880.0f32, 1108.7, 1318.5, 1760.0, 2217.5, 2637.0]
                .iter()
                .enumerate()
            {
                note(&mut s, 0.12 + k as f32 * 0.07, *f, 0.18, 0.32);
            }
            thump(&mut s, 0.5, 140.0, 0.12, 0.5);
        }
    }
    normalise(s, 0.9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_sound_is_short_audible_and_clean() {
        for sfx in UiSfx::ALL {
            let s = synthesise(sfx);
            assert!(!s.is_empty() && s.len() < RATE as usize * 2, "{sfx:?}");
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.5 && peak <= 0.91, "{sfx:?} peak {peak}");
            assert!(s.iter().all(|v| v.is_finite()), "{sfx:?}");
            assert!(s.last().unwrap().abs() < 0.01, "{sfx:?} ends with a click");
        }
    }
}
