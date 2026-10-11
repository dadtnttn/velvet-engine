//! Sound effects and music for VS3 apps.
//!
//! The manifest lists WAV files (`"sounds": [{"id", "path"}]`, 16-bit PCM or
//! 32-bit float, mono or stereo, any rate). Scripts emit:
//! - `audio.play {sound, volume?, rate?}`: one-shot effect (`rate` changes pitch);
//! - `audio.music {sound, volume?, fade?}`: loop a track (`""` stops), crossfading;
//! - `audio.volume {master?, sfx?, music?}`: mixer levels from 0 to 1.
//!
//! With the `audio` feature an output stream is opened with cpal; without it
//! (or without a device) every call is accepted and silently ignored.

#![cfg_attr(not(feature = "audio"), allow(dead_code))]

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::assets::SoundSpec;

/// Decoded audio: interleaved f32 samples.
#[derive(Clone)]
pub struct Clip {
    pub samples: Arc<Vec<f32>>,
    pub channels: u16,
    pub rate: u32,
}

/// Parse a RIFF/WAVE file (PCM 16-bit, PCM 8-bit or IEEE float 32-bit).
pub fn decode_wav(bytes: &[u8]) -> Option<Clip> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    let (mut format, mut channels, mut rate, mut bits) = (0u16, 0u16, 0u32, 0u16);
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body_end = (pos + 8 + size).min(bytes.len());
        let body = &bytes[pos + 8..body_end];
        if id == b"fmt " && body.len() >= 16 {
            format = u16::from_le_bytes([body[0], body[1]]);
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
            bits = u16::from_le_bytes([body[14], body[15]]);
            if format == 0xFFFE && body.len() >= 26 {
                format = u16::from_le_bytes([body[24], body[25]]);
            }
        } else if id == b"data" {
            data = Some(body);
        }
        pos = pos + 8 + size + (size & 1);
    }
    let data = data?;
    if channels == 0 || rate == 0 {
        return None;
    }
    let samples: Vec<f32> = match (format, bits) {
        (1, 16) => data
            .chunks_exact(2)
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
            .collect(),
        (1, 8) => data
            .iter()
            .map(|b| (f32::from(*b) - 128.0) / 128.0)
            .collect(),
        (3, 32) => data
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        _ => return None,
    };
    Some(Clip {
        samples: Arc::new(samples),
        channels,
        rate,
    })
}

struct Voice {
    clip: Clip,
    pos: f64,
    step: f64,
    volume: f32,
    looped: bool,
    /// Current fade gain and its change per output frame.
    gain: f32,
    gain_step: f32,
    music: bool,
    done: bool,
}

#[derive(Default)]
struct MixState {
    voices: Vec<Voice>,
    master: f32,
    sfx: f32,
    music: f32,
    out_rate: u32,
}

impl MixState {
    fn mix(&mut self, out: &mut [f32], channels: usize) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let frames = out.len() / channels.max(1);
        for v in self.voices.iter_mut() {
            let src_ch = v.clip.channels as usize;
            let total = v.clip.samples.len() / src_ch.max(1);
            let bus = if v.music { self.music } else { self.sfx };
            for f in 0..frames {
                let idx = v.pos as usize;
                if idx >= total {
                    if v.looped && total > 0 {
                        v.pos -= total as f64;
                    } else {
                        v.done = true;
                        break;
                    }
                }
                let idx = (v.pos as usize).min(total.saturating_sub(1));
                let l = v.clip.samples[idx * src_ch];
                let r = if src_ch > 1 {
                    v.clip.samples[idx * src_ch + 1]
                } else {
                    l
                };
                let g = v.volume * v.gain * bus * self.master;
                if channels >= 2 {
                    out[f * channels] += l * g;
                    out[f * channels + 1] += r * g;
                } else {
                    out[f * channels] += (l + r) * 0.5 * g;
                }
                v.pos += v.step;
                v.gain = (v.gain + v.gain_step).clamp(0.0, 1.0);
                if v.gain_step < 0.0 && v.gain <= 0.0 {
                    v.done = true;
                    break;
                }
            }
        }
        self.voices.retain(|v| !v.done);
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

pub struct Audio {
    clips: HashMap<String, Clip>,
    state: Arc<Mutex<MixState>>,
    #[cfg(feature = "audio")]
    _stream: Option<cpal::Stream>,
    current_music: String,
}

impl Audio {
    pub fn load(root: &Path, sounds: &[SoundSpec]) -> Self {
        let mut clips = HashMap::new();
        for s in sounds {
            match std::fs::read(root.join(&s.path)) {
                Ok(bytes) => match decode_wav(&bytes) {
                    Some(c) => {
                        clips.insert(s.id.clone(), c);
                    }
                    None => eprintln!("[velvet-vs3-app] sound `{}`: unsupported WAV", s.id),
                },
                Err(e) => eprintln!("[velvet-vs3-app] sound `{}`: {e}", s.id),
            }
        }
        let state = Arc::new(Mutex::new(MixState {
            voices: Vec::new(),
            master: 0.8,
            sfx: 1.0,
            music: 0.6,
            out_rate: 44_100,
        }));
        #[cfg(feature = "audio")]
        let stream = if clips.is_empty() {
            None
        } else {
            open_stream(state.clone())
        };
        Self {
            clips,
            state,
            #[cfg(feature = "audio")]
            _stream: stream,
            current_music: String::new(),
        }
    }

    fn voice(&self, id: &str, volume: f32, rate: f32, music: bool, fade: f32) -> Option<Voice> {
        let clip = self.clips.get(id)?.clone();
        let out_rate = self.state.lock().map(|s| s.out_rate).unwrap_or(44_100);
        let frames_fade = (fade.max(0.0) * out_rate as f32).max(1.0);
        Some(Voice {
            step: f64::from(clip.rate) / f64::from(out_rate) * f64::from(rate.clamp(0.25, 4.0)),
            clip,
            pos: 0.0,
            volume: volume.clamp(0.0, 2.0),
            looped: music,
            gain: if music && fade > 0.0 { 0.0 } else { 1.0 },
            gain_step: if music && fade > 0.0 {
                1.0 / frames_fade
            } else {
                0.0
            },
            music,
            done: false,
        })
    }

    pub fn play(&mut self, id: &str, volume: f32, rate: f32) {
        if let Some(v) = self.voice(id, volume, rate, false, 0.0) {
            if let Ok(mut s) = self.state.lock() {
                // Keep the mix sane when many effects pile up.
                if s.voices.iter().filter(|v| !v.music).count() < 24 {
                    s.voices.push(v);
                }
            }
        }
    }

    pub fn music(&mut self, id: &str, volume: f32, fade: f32) {
        if id == self.current_music {
            if let Ok(mut s) = self.state.lock() {
                for v in s.voices.iter_mut().filter(|v| v.music) {
                    v.volume = volume;
                }
            }
            return;
        }
        let out_rate = self.state.lock().map(|s| s.out_rate).unwrap_or(44_100);
        let frames_fade = (fade.max(0.05) * out_rate as f32).max(1.0);
        if let Ok(mut s) = self.state.lock() {
            for v in s.voices.iter_mut().filter(|v| v.music) {
                v.gain_step = -1.0 / frames_fade;
            }
        }
        self.current_music = id.to_owned();
        if id.is_empty() {
            return;
        }
        if let Some(v) = self.voice(id, volume, 1.0, true, fade.max(0.05)) {
            if let Ok(mut s) = self.state.lock() {
                s.voices.push(v);
            }
        }
    }

    pub fn volume(&mut self, master: Option<f32>, sfx: Option<f32>, music: Option<f32>) {
        if let Ok(mut s) = self.state.lock() {
            if let Some(m) = master {
                s.master = m.clamp(0.0, 1.0);
            }
            if let Some(m) = sfx {
                s.sfx = m.clamp(0.0, 1.0);
            }
            if let Some(m) = music {
                s.music = m.clamp(0.0, 1.0);
            }
        }
    }

    /// Render `frames` stereo frames offline.
    #[cfg(test)]
    pub fn render_offline(&self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        if let Ok(mut s) = self.state.lock() {
            s.mix(&mut out, 2);
        }
        out
    }
}

#[cfg(feature = "audio")]
fn open_stream(state: Arc<Mutex<MixState>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    let channels = config.channels() as usize;
    if let Ok(mut s) = state.lock() {
        s.out_rate = config.sample_rate().0;
    }
    let err = |e| eprintln!("[velvet-vs3-app] audio stream: {e}");
    let stream_config: cpal::StreamConfig = config.clone().into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                if let Ok(mut s) = state.lock() {
                    s.mix(data, channels);
                }
            },
            err,
            None,
        ),
        cpal::SampleFormat::I16 => {
            let mut buf = Vec::new();
            device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| {
                    buf.resize(data.len(), 0.0f32);
                    if let Ok(mut s) = state.lock() {
                        s.mix(&mut buf, channels);
                    }
                    for (o, i) in data.iter_mut().zip(&buf) {
                        *o = (i * 32767.0) as i16;
                    }
                },
                err,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let mut buf = Vec::new();
            device.build_output_stream(
                &stream_config,
                move |data: &mut [u16], _| {
                    buf.resize(data.len(), 0.0f32);
                    if let Ok(mut s) = state.lock() {
                        s.mix(&mut buf, channels);
                    }
                    for (o, i) in data.iter_mut().zip(&buf) {
                        *o = ((i * 0.5 + 0.5) * 65535.0) as u16;
                    }
                },
                err,
                None,
            )
        }
        _ => return None,
    }
    .ok()?;
    stream.play().ok()?;
    Some(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
        let data_len = samples.len() * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples {
            b.extend_from_slice(&s.to_le_bytes());
        }
        b
    }

    #[test]
    fn decodes_and_mixes() {
        let clip = decode_wav(&wav(&[16384; 100], 44_100)).expect("wav");
        assert_eq!(clip.channels, 1);
        assert_eq!(clip.samples.len(), 100);
        let mut a = Audio {
            clips: HashMap::from([("beep".to_string(), clip)]),
            state: Arc::new(Mutex::new(MixState {
                voices: Vec::new(),
                master: 1.0,
                sfx: 1.0,
                music: 1.0,
                out_rate: 44_100,
            })),
            #[cfg(feature = "audio")]
            _stream: None,
            current_music: String::new(),
        };
        a.play("beep", 1.0, 1.0);
        let out = a.render_offline(50);
        assert!((out[0] - 0.5).abs() < 0.01);
        let rest = a.render_offline(100);
        assert!(rest[120].abs() < 1e-6, "voice ends after its samples");
    }
}
