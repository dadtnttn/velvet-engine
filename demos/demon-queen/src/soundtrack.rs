//! The soundtrack: four places, each with its own numbered playlist of long
//! tracks (`data/audio/music/<zone>_<n>.ogg`, listed with their lengths in
//! `manifest.json` by `tools/convert_music.py`). Tracks of a zone follow one
//! another in order with a long crossfade; moving between zones fades one
//! playlist out while the next fades in. It sits under everything else and
//! ducks a little whenever an effect plays, so effects always read clearly.
use rodio::mixer::Mixer;
use rodio::{Decoder, Sink};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Where the player is, as far as the soundtrack cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Zone {
    Castle,
    Forest,
    Village,
    Cave,
    Brothel,
}

impl Zone {
    pub(crate) const ALL: [Zone; 5] = [
        Zone::Castle,
        Zone::Forest,
        Zone::Village,
        Zone::Cave,
        Zone::Brothel,
    ];

    /// Key of the zone in the manifest and in file names.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Zone::Castle => "castle",
            Zone::Forest => "forest",
            Zone::Village => "village",
            Zone::Cave => "cave",
            Zone::Brothel => "brothel",
        }
    }
}

/// Seconds a track overlaps the next one of its zone.
pub(crate) const CROSSFADE: f32 = 7.0;
/// Seconds to swap one zone's music for another's.
pub(crate) const ZONE_FADE: f32 = 3.5;
/// Loudness of the music before ducking: it stays under the game.
pub(crate) const MUSIC_VOLUME: f32 = 0.34;
/// How far effects push the music down (1.0 would silence it).
pub(crate) const DUCK_DEPTH: f32 = 0.5;

/// A gain that eases in and out without a dip in loudness when two tracks
/// cross (the two fades add up to constant power).
pub(crate) fn equal_power(x: f32) -> f32 {
    (x.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2).sin()
}

/// Moves `current` toward `target` by at most `rate` per second.
pub(crate) fn approach(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let step = rate.max(0.0) * dt.max(0.0);
    if (target - current).abs() <= step {
        target
    } else {
        current + step.copysign(target - current)
    }
}

/// The gain the music takes for the loudest recent effect (0..1) and for the
/// pause menu.
pub(crate) fn duck_gain(effect_level: f32, paused: bool) -> f32 {
    let ducked = 1.0 - DUCK_DEPTH * effect_level.clamp(0.0, 1.0);
    ducked * if paused { 0.4 } else { 1.0 }
}

/// Whether the track that has played `age` seconds of `seconds` should hand
/// over to the next now. Very short tracks just play out.
pub(crate) fn handoff_due(age: f32, seconds: f32) -> bool {
    seconds > CROSSFADE * 2.0 && age >= seconds - CROSSFADE
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Track {
    pub path: PathBuf,
    pub seconds: f32,
}

/// A zone's tracks in play order, remembering which is next so coming back
/// to a place carries on with the following song.
#[derive(Clone, Debug, Default)]
pub(crate) struct Playlist {
    tracks: Vec<Track>,
    next: usize,
}

impl Playlist {
    pub(crate) fn new(tracks: Vec<Track>) -> Self {
        Self { tracks, next: 0 }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
    /// The next track, wrapping round after the last.
    pub(crate) fn take_next(&mut self) -> Option<Track> {
        if self.tracks.is_empty() {
            return None;
        }
        let track = self.tracks[self.next % self.tracks.len()].clone();
        self.next = (self.next + 1) % self.tracks.len();
        Some(track)
    }
}

#[derive(Deserialize)]
struct ManifestTrack {
    file: String,
    seconds: f32,
}

/// Reads `manifest.json` in `dir`; tracks whose file is missing are left out.
pub(crate) fn load_playlists(dir: &Path) -> BTreeMap<Zone, Playlist> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(dir.join("manifest.json")) else {
        return out;
    };
    let Ok(manifest) = serde_json::from_str::<BTreeMap<String, Vec<ManifestTrack>>>(&text) else {
        eprintln!("[music] manifest.json is not readable");
        return out;
    };
    for zone in Zone::ALL {
        let tracks: Vec<Track> = manifest
            .get(zone.key())
            .into_iter()
            .flatten()
            .map(|t| Track {
                path: dir.join(&t.file),
                seconds: t.seconds,
            })
            .filter(|t| t.path.is_file())
            .collect();
        if !tracks.is_empty() {
            out.insert(zone, Playlist::new(tracks));
        }
    }
    out
}

/// Where the music lives (`data/audio/music`).
pub(crate) fn music_dir() -> PathBuf {
    crate::exploration3d_state::data_dir().join("audio/music")
}

/// One playing track and how loud it currently is.
struct Voice {
    sink: Sink,
    zone: Zone,
    started: Instant,
    seconds: f32,
    fade: f32,
    target: f32,
    fade_seconds: f32,
    handed_off: bool,
}

pub(crate) struct Music {
    mixer: Mixer,
    playlists: BTreeMap<Zone, Playlist>,
    voices: Vec<Voice>,
    zone: Option<Zone>,
    duck: f32,
    /// How loud this soundtrack is against the game's (1: the world's; the visual novel's is
    /// lower).
    level: f32,
}

impl Music {
    /// `None` when there is no soundtrack on disk.
    pub(crate) fn new(mixer: Mixer, dir: &Path) -> Option<Self> {
        let playlists = load_playlists(dir);
        if playlists.is_empty() {
            eprintln!("[music] no soundtrack found in {}", dir.display());
            return None;
        }
        let count: usize = playlists.values().map(|p| p.tracks.len()).sum();
        eprintln!("[music] {count} tracks in {} zones", playlists.len());
        Some(Self {
            mixer,
            playlists,
            voices: Vec::new(),
            zone: None,
            duck: 1.0,
            level: 1.0,
        })
    }

    /// Plays at `level` times the world's loudness.
    pub(crate) fn set_level(&mut self, level: f32) {
        self.level = level.clamp(0.0, 1.0);
    }

    /// Fades everything out (over a second and a half); whether anything still sounds.
    pub(crate) fn fade_out(&mut self, dt: f32) -> bool {
        let dt = dt.clamp(0.0, 0.25);
        for voice in &mut self.voices {
            if voice.target > 0.0 {
                voice.target = 0.0;
                voice.fade_seconds = 1.5;
            }
            voice.fade = approach(voice.fade, 0.0, 1.0 / voice.fade_seconds, dt);
            voice.sink.set_volume(
                MUSIC_VOLUME
                    * self.level
                    * self.duck
                    * equal_power(voice.fade)
                    * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Music),
            );
        }
        self.voices.retain(|v| v.fade > 0.001);
        // Back in, it starts its zone afresh.
        if self.voices.is_empty() {
            self.zone = None;
        }
        !self.voices.is_empty()
    }

    fn start(&mut self, zone: Zone, fade_seconds: f32) -> bool {
        // A track that will not open is skipped for the next one.
        for _ in 0..self.playlists.get(&zone).map_or(0, |p| p.tracks.len()) {
            let Some(track) = self.playlists.get_mut(&zone).and_then(Playlist::take_next) else {
                return false;
            };
            let file = match std::fs::File::open(&track.path) {
                Ok(file) => file,
                Err(error) => {
                    eprintln!("[music] {}: {error}", track.path.display());
                    continue;
                }
            };
            match Decoder::new(BufReader::new(file)) {
                Ok(decoder) => {
                    let sink = Sink::connect_new(&self.mixer);
                    sink.set_volume(0.0);
                    sink.append(decoder);
                    self.voices.push(Voice {
                        sink,
                        zone,
                        started: Instant::now(),
                        seconds: track.seconds,
                        fade: 0.0,
                        target: 1.0,
                        fade_seconds: fade_seconds.max(0.1),
                        handed_off: false,
                    });
                    return true;
                }
                Err(error) => eprintln!("[music] {}: {error}", track.path.display()),
            }
        }
        false
    }

    /// Each frame: follow the zone, hand tracks over, fade and duck.
    /// `effect_level` (0..1) is how loud the effects of the last moments were.
    pub(crate) fn update(&mut self, zone: Zone, dt: f32, effect_level: f32, paused: bool) {
        let dt = dt.clamp(0.0, 0.25);
        if self.zone != Some(zone) {
            self.zone = Some(zone);
            for voice in self.voices.iter_mut().filter(|v| v.zone != zone) {
                voice.target = 0.0;
                voice.fade_seconds = ZONE_FADE;
            }
            // Coming straight back to a zone still fading out resumes it.
            let resumed = self
                .voices
                .iter_mut()
                .find(|v| v.zone == zone && v.target == 0.0 && v.fade > 0.2);
            match resumed {
                Some(voice) => {
                    voice.target = 1.0;
                    voice.fade_seconds = ZONE_FADE;
                }
                None => {
                    self.start(
                        zone,
                        if self.voices.is_empty() {
                            2.0
                        } else {
                            ZONE_FADE
                        },
                    );
                }
            }
        }
        // Tracks that ended (or failed) are replaced; tracks near their end hand over.
        self.voices.retain(|v| !v.sink.empty());
        let live = self.voices.iter().any(|v| v.zone == zone && v.target > 0.0);
        if !live {
            self.start(zone, 1.5);
        }
        let due: Vec<usize> = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                v.zone == zone
                    && v.target > 0.0
                    && !v.handed_off
                    && handoff_due(v.started.elapsed().as_secs_f32(), v.seconds)
            })
            .map(|(i, _)| i)
            .collect();
        for i in due {
            if self.start(zone, CROSSFADE) {
                let voice = &mut self.voices[i];
                voice.handed_off = true;
                voice.target = 0.0;
                voice.fade_seconds = CROSSFADE;
            }
        }
        // Effects push the music down quickly and let it back up slowly.
        let want = duck_gain(effect_level, paused);
        let rate = if want < self.duck { 6.0 } else { 0.7 };
        self.duck = approach(self.duck, want, rate, dt);
        for voice in &mut self.voices {
            voice.fade = approach(voice.fade, voice.target, 1.0 / voice.fade_seconds, dt);
            voice
                .sink
                .set_volume(MUSIC_VOLUME * self.level * self.duck * equal_power(voice.fade) * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Music));
        }
        self.voices
            .retain(|v| !(v.target == 0.0 && v.fade <= 0.001));
    }

    /// Lowers (or restores) the whole soundtrack at once, for the pause menu.
    pub(crate) fn hush(&mut self, paused: bool) {
        self.duck = duck_gain(0.0, paused);
        for voice in &self.voices {
            voice
                .sink
                .set_volume(MUSIC_VOLUME * self.level * self.duck * equal_power(voice.fade) * crate::exploration3d::prefs::gain(crate::exploration3d::prefs::Channel::Music));
        }
    }

    /// Tracks currently sounding, for tests and logs.
    #[allow(dead_code)]
    pub(crate) fn playing(&self) -> usize {
        self.voices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracks(n: usize) -> Vec<Track> {
        (1..=n)
            .map(|k| Track {
                path: PathBuf::from(format!("castle_{k}.ogg")),
                seconds: 300.0 + k as f32,
            })
            .collect()
    }

    #[test]
    fn a_playlist_plays_in_order_and_wraps_and_remembers_where_it_was() {
        let mut list = Playlist::new(tracks(3));
        let order: Vec<String> = (0..7)
            .map(|_| {
                list.take_next()
                    .unwrap()
                    .path
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            order,
            [
                "castle_1.ogg",
                "castle_2.ogg",
                "castle_3.ogg",
                "castle_1.ogg",
                "castle_2.ogg",
                "castle_3.ogg",
                "castle_1.ogg"
            ]
        );
        assert!(Playlist::default().take_next().is_none());
        assert!(Playlist::default().is_empty());
    }

    #[test]
    fn crossfading_keeps_the_power_constant_and_never_jumps() {
        for k in 0..=100 {
            let x = k as f32 / 100.0;
            let power = equal_power(x).powi(2) + equal_power(1.0 - x).powi(2);
            assert!((power - 1.0).abs() < 1e-4, "constant power at {x}: {power}");
        }
        assert_eq!(equal_power(0.0), 0.0);
        assert!((equal_power(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(equal_power(-3.0), 0.0);
        assert!(equal_power(0.4) < equal_power(0.6));
    }

    #[test]
    fn tracks_hand_over_seven_seconds_before_they_end_and_short_ones_play_out() {
        assert!(!handoff_due(100.0, 360.0));
        assert!(!handoff_due(352.9, 360.0));
        assert!(handoff_due(353.0, 360.0));
        assert!(handoff_due(400.0, 360.0));
        assert!(!handoff_due(5.0, 10.0), "a ten second sting is left alone");
    }

    #[test]
    fn approach_reaches_its_target_without_overshooting() {
        assert_eq!(approach(0.0, 1.0, 0.5, 1.0), 0.5);
        assert_eq!(approach(0.9, 1.0, 0.5, 1.0), 1.0);
        assert_eq!(approach(1.0, 0.0, 2.0, 0.25), 0.5);
        assert_eq!(approach(0.3, 0.3, 1.0, 1.0), 0.3);
        assert_eq!(approach(0.0, 1.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn effects_duck_the_music_and_the_pause_menu_lowers_it_more() {
        assert_eq!(duck_gain(0.0, false), 1.0);
        assert!((duck_gain(1.0, false) - (1.0 - DUCK_DEPTH)).abs() < 1e-6);
        assert!(duck_gain(0.5, false) > duck_gain(1.0, false));
        assert!(duck_gain(0.0, true) < 0.5);
        assert!(duck_gain(9.0, false) >= 1.0 - DUCK_DEPTH, "clamped");
        // Effects stay clearly above the music even at full duck.
        assert!(MUSIC_VOLUME * duck_gain(1.0, false) < 0.25);
    }

    #[test]
    fn the_shipped_manifest_lists_every_track_in_order_and_the_files_exist() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/audio/music");
        if !dir.join("manifest.json").is_file() {
            return; // the soundtrack is not part of every checkout
        }
        let lists = load_playlists(&dir);
        for (zone, expected) in [
            (Zone::Castle, 3),
            (Zone::Forest, 4),
            (Zone::Village, 4),
            (Zone::Cave, 6),
        ] {
            let mut list = lists
                .get(&zone)
                .unwrap_or_else(|| panic!("{zone:?} has no playlist"))
                .clone();
            assert_eq!(list.tracks.len(), expected, "{zone:?}");
            for k in 1..=expected {
                let track = list.take_next().unwrap();
                assert!(
                    track.path.ends_with(format!("{}_{k}.ogg", zone.key())),
                    "{zone:?} track {k} is {:?}",
                    track.path
                );
                assert!(track.seconds > 120.0, "{zone:?} #{k} is a full song");
            }
        }
    }

    #[test]
    fn a_missing_manifest_or_file_just_means_no_music() {
        let empty = std::env::temp_dir().join("demon-queen-no-music");
        let _ = std::fs::create_dir_all(&empty);
        assert!(load_playlists(&empty).is_empty());
        std::fs::write(
            empty.join("manifest.json"),
            r#"{"castle":[{"file":"gone.ogg","seconds":300.0}]}"#,
        )
        .unwrap();
        assert!(
            load_playlists(&empty).is_empty(),
            "files that are not there are skipped"
        );
        std::fs::write(empty.join("manifest.json"), "not json").unwrap();
        assert!(load_playlists(&empty).is_empty());
    }

    #[test]
    fn every_shipped_track_decodes_and_reports_its_format() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/audio/music");
        if !dir.join("manifest.json").is_file() {
            return;
        }
        use rodio::Source;
        for (_, mut list) in load_playlists(&dir) {
            while let Some(track) = list.take_next() {
                let file = std::fs::File::open(&track.path).unwrap();
                let decoder = Decoder::new(BufReader::new(file))
                    .unwrap_or_else(|e| panic!("{}: {e}", track.path.display()));
                assert_eq!(decoder.channels(), 2, "{}", track.path.display());
                assert_eq!(decoder.sample_rate(), 44_100, "{}", track.path.display());
                if list.next == 0 {
                    break;
                }
            }
        }
    }
}
