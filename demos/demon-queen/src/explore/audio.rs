//! Lo que se oye arriba: la música y el ambiente de cada lugar, los pasos y los sonidos de la interfaz.
use super::*;

impl App {
    /// Which place the soundtrack and the ambience should be playing for.
    pub(super) fn audio_zone(&self) -> soundtrack::Zone {
        self.zone().soundtrack(self)
    }

    /// 0 by day, 1 at night, from the sky's own clock.
    pub(super) fn night_factor(&self) -> f32 {
        let moon = self.environment_cycle.sample().celestial.moon_visibility;
        ((moon - 0.12) / 0.48).clamp(0.0, 1.0)
    }

    /// Takes the sounds made on the background thread once they are ready.
    pub(super) fn poll_audio_boot(&mut self) {
        if self
            .audio_boot
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            if let Some(handle) = self.audio_boot.take() {
                self.audio = handle
                    .join()
                    .ok()
                    .and_then(game_audio::GameAudio::from_synth);
            }
        }
    }

    pub(super) fn update_audio_above_ground(&mut self, dt: f32) {
        // Only the real game opens a sound device: not tests or captures. The
        // sounds are made on another thread, so the world never hitches.
        if !self.audio_tried && self.window.is_some() && std::env::var_os("VELVET_MUTE").is_none() {
            self.audio_tried = true;
            self.audio_boot = std::thread::Builder::new()
                .name("audio-synth".into())
                .spawn(game_audio::Synth::build)
                .ok();
        }
        self.poll_audio_boot();
        let (sin_yaw, cos_yaw) = self.iso.yaw.sin_cos();
        let scene = game_audio::Scene {
            emitters: self.zone().emitters(self),
            zone: self.audio_zone(),
            night: self.night_factor(),
            listener: self.player,
            forward: Vec3::new(-sin_yaw, 0.0, -cos_yaw),
            paused: false,
            springs: self.zone().springs(self),
        };
        if let Some(audio) = &mut self.audio {
            audio.update_scene(scene, 0.0, dt);
        }
    }

    /// One audible step, matching the ground under the Queen.
    pub(super) fn play_step(&mut self) {
        // At the springs the ground underfoot decides: water, boards, stone, grass.
        let ground = self.zone().ground(self);
        let sfx = ground.sfx();
        let volume = if self.sprint { 0.62 } else { 0.42 };
        let listener = self.player;
        if let Some(audio) = &mut self.audio {
            audio.play(
                &dungeon::sound::Sound {
                    sfx,
                    at: None,
                    volume,
                },
                listener,
                Vec3::Z,
            );
        }
    }

    /// A menu-level sound (coins on a sale), heard at full presence.
    pub(super) fn ui_sound(&mut self, sfx: dungeon::sound::Sfx, volume: f32) {
        let listener = self.player;
        if let Some(audio) = &mut self.audio {
            audio.play(
                &dungeon::sound::Sound {
                    sfx,
                    at: None,
                    volume,
                },
                listener,
                Vec3::Z,
            );
        }
    }
}
