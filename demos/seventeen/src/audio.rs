use crate::model::EventView;
use crate::save::Settings;

#[cfg(windows)]
mod platform {
    use std::time::Duration;

    use rodio::source::SineWave;
    use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};

    use super::{EventView, Settings};

    pub struct Audio {
        stream: Option<OutputStream>,
        ambience: Option<Sink>,
        warned: bool,
    }

    impl Audio {
        pub fn new(settings: &Settings) -> Self {
            match OutputStreamBuilder::open_default_stream() {
                Ok(stream) => {
                    let ambience = Sink::connect_new(stream.mixer());
                    ambience.append(SineWave::new(43.0).amplify(0.035).repeat_infinite());
                    ambience.set_volume(settings.master_volume * settings.music_volume * 0.65);
                    Self {
                        stream: Some(stream),
                        ambience: Some(ambience),
                        warned: false,
                    }
                }
                Err(error) => {
                    eprintln!("17: audio unavailable ({error})");
                    Self {
                        stream: None,
                        ambience: None,
                        warned: true,
                    }
                }
            }
        }

        pub fn update_settings(&self, settings: &Settings) {
            if let Some(ambience) = &self.ambience {
                ambience.set_volume(settings.master_volume * settings.music_volume * 0.65);
            }
        }

        pub fn play_events(&mut self, events: &[EventView], settings: &Settings) {
            let Some(stream) = &self.stream else {
                return;
            };
            let volume = settings.master_volume * settings.effects_volume;
            if volume <= 0.001 {
                return;
            }
            for event in events {
                let (frequency, milliseconds, amplitude) = match event.kind.as_str() {
                    "pistol" => (172.0, 72, 0.18),
                    "shotgun" => (73.0, 145, 0.28),
                    "blade" => (610.0, 90, 0.12),
                    "deflect" => (940.0, 75, 0.13),
                    "enemy_shot" => (116.0, 95, 0.11),
                    "impact" => (245.0, 55, 0.10),
                    "enemy_down" => (82.0, 210, 0.15),
                    "player_hit" => (62.0, 170, 0.22),
                    "death" => (39.0, 600, 0.28),
                    "respawn" => (330.0, 380, 0.10),
                    "pickup" => (740.0, 220, 0.10),
                    "door" => (94.0, 260, 0.12),
                    "room_clear" => (440.0, 330, 0.08),
                    "room_enter" => (56.0, 180, 0.06),
                    "boss_phase" => (51.0, 720, 0.22),
                    "ending" => (220.0, 900, 0.10),
                    "dash" => (310.0, 45, 0.06),
                    "empty" => (880.0, 30, 0.05),
                    "reload" | "reload_complete" => (510.0, 55, 0.055),
                    "hound" => (69.0, 130, 0.16),
                    "spark" => (920.0, 25, 0.035),
                    _ => continue,
                };
                let sink = Sink::connect_new(stream.mixer());
                sink.set_volume(volume);
                sink.append(
                    SineWave::new(frequency)
                        .take_duration(Duration::from_millis(milliseconds))
                        .amplify(amplitude * event.power.clamp(0.35, 2.0))
                        .fade_out(Duration::from_millis((milliseconds / 2).max(10))),
                );
                sink.detach();
            }
        }

        pub fn play_ui(&mut self, confirm: bool, settings: &Settings) {
            let Some(stream) = &self.stream else {
                return;
            };
            let sink = Sink::connect_new(stream.mixer());
            sink.set_volume(settings.master_volume * settings.effects_volume);
            sink.append(
                SineWave::new(if confirm { 660.0 } else { 420.0 })
                    .take_duration(Duration::from_millis(if confirm { 70 } else { 35 }))
                    .amplify(0.06),
            );
            sink.detach();
        }

        pub fn is_available(&self) -> bool {
            self.stream.is_some() && !self.warned
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
        pub fn update_settings(&self, _: &Settings) {}
        pub fn play_events(&mut self, _: &[EventView], _: &Settings) {}
        pub fn play_ui(&mut self, _: bool, _: &Settings) {}
        pub fn is_available(&self) -> bool {
            false
        }
    }
}

pub use platform::Audio;
