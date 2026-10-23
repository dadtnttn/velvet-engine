//! Plays sounds through the default output. Silent, never failing, without a sound device or with `VELVET_MUTE`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamBuilder, Sink};

use crate::{Sound, RATE};

/// The sound output of a game.
pub struct Player {
    /// Never closed: at the end of the program Windows has already stopped the stream's thread, and closing it then
    /// waits on a thread that is gone (found in Demon Queen).
    stream: Option<std::mem::ManuallyDrop<OutputStream>>,
    /// The last time each named sound was played, so one held key does not pile copies on each other.
    last: HashMap<String, Instant>,
    /// The volume of every sound, from 0 to 1.
    pub volume: f32,
}

impl Player {
    /// Opens the default output (or none, quietly).
    pub fn new() -> Player {
        let stream = if std::env::var_os("VELVET_MUTE").is_some() {
            None
        } else {
            match OutputStreamBuilder::open_default_stream() {
                Ok(mut stream) => {
                    stream.log_on_drop(false);
                    Some(std::mem::ManuallyDrop::new(stream))
                }
                Err(error) => {
                    eprintln!("[velvet-sfx] no sound output ({error}): the game stays silent");
                    None
                }
            }
        };
        Player {
            stream,
            last: HashMap::new(),
            volume: 1.0,
        }
    }

    /// True when sounds are heard.
    pub fn has_output(&self) -> bool {
        self.stream.is_some()
    }

    /// Plays a sound at a volume (times the player's), unless the same `name` played less than `spacing` ago.
    pub fn play(&mut self, name: &str, sound: &Sound, volume: f32, spacing: Duration) {
        let Some(stream) = &self.stream else { return };
        let now = Instant::now();
        if self
            .last
            .get(name)
            .is_some_and(|t| now.duration_since(*t) < spacing)
        {
            return;
        }
        self.last.insert(name.to_string(), now);
        let sink = Sink::connect_new(stream.mixer());
        sink.append(SamplesBuffer::new(1, RATE, sound.samples.clone()));
        sink.set_volume((volume * self.volume).clamp(0.0, 1.0));
        sink.detach();
    }
}

impl Default for Player {
    fn default() -> Self {
        Player::new()
    }
}
