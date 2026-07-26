use crate::model::EventView;
use crate::save::Settings;

#[cfg(windows)]
mod platform {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;
    use std::time::Duration;

    use rodio::source::SineWave;
    use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

    use super::{EventView, Settings};

    pub struct Audio {
        stream: Option<OutputStream>,
        sounds_dir: PathBuf,
    }

    impl Audio {
        pub fn new(_settings: &Settings) -> Self {
            let sounds_dir = PathBuf::from("demos/echo-xp/data/assets/sounds");
            match OutputStreamBuilder::open_default_stream() {
                Ok(stream) => Self {
                    stream: Some(stream),
                    sounds_dir,
                },
                Err(error) => {
                    eprintln!("echo-xp: audio output stream unavailable ({error})");
                    Self {
                        stream: None,
                        sounds_dir,
                    }
                }
            }
        }

        pub fn play_sound(&mut self, name: &str, settings: &Settings) {
            let Some(stream) = &self.stream else {
                return;
            };
            let volume = settings.master_volume * settings.effects_volume;
            if volume <= 0.001 {
                return;
            }

            let sound_file = match name {
                "startup" => "startup.wav",
                "logon" => "logon.wav",
                "shutdown" => "shutdown.wav",
                "ding" => "ding.wav",
                "error" => "error.wav",
                "warning" => "warning.wav",
                "critical" => "critical.wav",
                "menu" => "menu.wav",
                "minimize" => "minimize.wav",
                "restore" => "restore.wav",
                "notify" => "notify.wav",
                _ => "",
            };

            let wav_path = self.sounds_dir.join(sound_file);
            if !sound_file.is_empty() && wav_path.exists() {
                if let Ok(file) = File::open(&wav_path) {
                    if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                        let sink = Sink::connect_new(stream.mixer());
                        sink.set_volume(volume * 0.7);
                        sink.append(decoder);
                        sink.detach();
                        return;
                    }
                }
            }

            // Fallback synthesized sound tone if WAV fails or missing
            let (freq, ms) = match name {
                "startup" | "logon" => (440.0, 300),
                "error" | "critical" => (150.0, 250),
                "warning" => (350.0, 180),
                "notify" | "ding" => (880.0, 120),
                "menu" => (520.0, 40),
                _ => (400.0, 60),
            };

            let sink = Sink::connect_new(stream.mixer());
            sink.set_volume(volume * 0.2);
            sink.append(SineWave::new(freq).take_duration(Duration::from_millis(ms)));
            sink.detach();
        }

        pub fn play_events(&mut self, events: &[EventView], settings: &Settings) {
            for ev in events {
                if ev.kind == "sound" {
                    self.play_sound(&ev.name, settings);
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{EventView, Settings};

    pub struct Audio;

    impl Audio {
        pub fn new(_: &Settings) -> Self {
            Self
        }
        pub fn play_sound(&mut self, _: &str, _: &Settings) {}
        pub fn play_events(&mut self, _: &[EventView], _: &Settings) {}
    }
}

pub use platform::Audio;
