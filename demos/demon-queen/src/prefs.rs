//! The player's preferences that the whole game reads as it runs: the volume of each kind of
//! sound, how fast lines type themselves out, the mouse's speed when looking around, and full
//! screen. They live in `settings.json` beside the language (the file's other keys are kept as
//! they are) and are read once at start; the menus change them here and save them.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use serde_json::{Map, Value};

/// Steps of every volume (0 = silence, `VOLUME_STEPS` = as mixed).
pub(crate) const VOLUME_STEPS: u8 = 10;
/// Steps of the mouse's speed; `SENSITIVITY_DEFAULT` is the speed the game was tuned at.
pub(crate) const SENSITIVITY_STEPS: u8 = 10;
pub(crate) const SENSITIVITY_DEFAULT: u8 = 5;
/// Text speeds: slow, normal, fast, instant.
pub(crate) const TEXT_SPEEDS: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Channel {
    /// Everything.
    Master,
    Music,
    /// Beds of wind, water, crowds and the world's own sounds.
    Ambience,
    /// Steps, blows, the interface's clicks.
    Effects,
    /// The sound of the scenes' clips and voices.
    Scenes,
}

impl Channel {
    pub(crate) const ALL: [Channel; 5] = [
        Channel::Master,
        Channel::Music,
        Channel::Ambience,
        Channel::Effects,
        Channel::Scenes,
    ];

    fn key(self) -> &'static str {
        match self {
            Channel::Master => "master",
            Channel::Music => "music",
            Channel::Ambience => "ambience",
            Channel::Effects => "effects",
            Channel::Scenes => "scenes",
        }
    }
}

static VOLUMES: [AtomicU8; 5] = [
    AtomicU8::new(VOLUME_STEPS),
    AtomicU8::new(VOLUME_STEPS),
    AtomicU8::new(VOLUME_STEPS),
    AtomicU8::new(VOLUME_STEPS),
    AtomicU8::new(VOLUME_STEPS),
];
static TEXT_SPEED: AtomicU8 = AtomicU8::new(1);
static SENSITIVITY: AtomicU8 = AtomicU8::new(SENSITIVITY_DEFAULT);
static FULLSCREEN: AtomicBool = AtomicBool::new(false);
static NATIVE_RESOLUTION: AtomicBool = AtomicBool::new(true);

pub(crate) fn volume(channel: Channel) -> u8 {
    VOLUMES[channel as usize].load(Ordering::Relaxed)
}

pub(crate) fn set_volume(channel: Channel, steps: u8) {
    VOLUMES[channel as usize].store(steps.min(VOLUME_STEPS), Ordering::Relaxed);
}

/// What a sound of `channel` is multiplied by: its slider and the general one, each squared so
/// that the steps sound even.
pub(crate) fn gain(channel: Channel) -> f32 {
    let curve = |steps: u8| {
        let k = steps as f32 / VOLUME_STEPS as f32;
        k * k
    };
    let own = if channel == Channel::Master {
        1.0
    } else {
        curve(volume(channel))
    };
    curve(volume(Channel::Master)) * own
}

pub(crate) fn text_speed() -> u8 {
    TEXT_SPEED.load(Ordering::Relaxed)
}

pub(crate) fn set_text_speed(speed: u8) {
    TEXT_SPEED.store(speed.min(TEXT_SPEEDS - 1), Ordering::Relaxed);
}

/// Characters a second for a line that types itself at `normal` characters a second.
pub(crate) fn text_rate(normal: f32) -> f32 {
    match text_speed() {
        0 => normal * 0.55,
        1 => normal,
        2 => normal * 1.9,
        _ => 100_000.0,
    }
}

pub(crate) fn sensitivity() -> u8 {
    SENSITIVITY.load(Ordering::Relaxed)
}

pub(crate) fn set_sensitivity(steps: u8) {
    SENSITIVITY.store(steps.clamp(1, SENSITIVITY_STEPS), Ordering::Relaxed);
}

/// What the mouse's movement is multiplied by when it turns the view (1 at the default step).
pub(crate) fn look_scale() -> f32 {
    const SCALE: [f32; SENSITIVITY_STEPS as usize] =
        [0.4, 0.55, 0.7, 0.85, 1.0, 1.2, 1.4, 1.65, 1.9, 2.2];
    SCALE[(sensitivity().clamp(1, SENSITIVITY_STEPS) - 1) as usize]
}

pub(crate) fn fullscreen() -> bool {
    FULLSCREEN.load(Ordering::Relaxed)
}

pub(crate) fn set_fullscreen(on: bool) {
    FULLSCREEN.store(on, Ordering::Relaxed);
}

/// The frame drawn at the screen's own resolution (sharp on any screen), else at the authored
/// 1920×1080 and scaled (the fallback, for a weak or troublesome GPU).
pub(crate) fn native_resolution() -> bool {
    NATIVE_RESOLUTION.load(Ordering::Relaxed)
}

pub(crate) fn set_native_resolution(on: bool) {
    NATIVE_RESOLUTION.store(on, Ordering::Relaxed);
}

/// `VELVET_RENDER_SIZE=<w>x<h>` (captures: the screen's pixels without a window).
pub(crate) fn forced_render_size() -> Option<(u32, u32)> {
    let value = std::env::var("VELVET_RENDER_SIZE").ok()?;
    let (w, h) = value.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

/// The size the frame is drawn at for a window of `window` pixels: with the native resolution,
/// the 16:9 area the window shows it in (between 640×360 and 3840×2160); else 1920×1080.
pub(crate) fn render_size(window: (u32, u32)) -> (u32, u32) {
    // QA: `VELVET_RENDER_SIZE=2560x1440` draws at that size whatever the window.
    if let Some((w, h)) = forced_render_size() {
        return (w, h);
    }
    if !native_resolution() || window.0 == 0 || window.1 == 0 {
        return (1920, 1080);
    }
    let width = (window.0 as f32).min(window.1 as f32 * 16.0 / 9.0);
    let width = width.clamp(640.0, 3840.0).round() as u32;
    (width, (width as f32 * 9.0 / 16.0).round() as u32)
}

/// The graphics settings (Settings → Graphics). `MAXIMUM` is the game's full look, the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Graphics {
    /// 0 low, 1 medium, 2 high, 3 maximum, 4 custom (any setting changed by hand).
    pub preset: u8,
    /// Sun shadows: 0 off, 1 low (4 samples), 2 medium (8), 3 high (16).
    pub shadows: u8,
    pub bounced_light: bool,
    pub reflections: bool,
    /// 0 off, 1 light, 2 full.
    pub bloom: u8,
    /// Render scale, percent of the shown size.
    pub render_scale: u8,
    /// 0 bilinear, 1 AMD FSR 1.
    pub upscaler: u8,
    /// Dynamic resolution: 0 off, 1 keeps 30 fps, 2 keeps 60 fps.
    pub dynamic: u8,
    /// 0 off, 1 FXAA.
    pub antialias: u8,
    /// Sharpening, 0..10.
    pub sharpness: u8,
    /// Detail of far things: 0 full, 1 balanced, 2 performance.
    pub lod: u8,
    /// Depth pre-pass (faster; not exact where surfaces overlap).
    pub prepass: bool,
}

pub(crate) const PRESET_CUSTOM: u8 = 4;
pub(crate) const RENDER_SCALES: [u8; 6] = [50, 59, 67, 75, 85, 100];

impl Graphics {
    pub(crate) const MAXIMUM: Graphics = Graphics {
        preset: 3,
        shadows: 3,
        bounced_light: true,
        reflections: true,
        bloom: 2,
        render_scale: 100,
        upscaler: 1,
        dynamic: 0,
        antialias: 0,
        sharpness: 0,
        lod: 0,
        prepass: false,
    };

    /// A preset's settings.
    pub(crate) fn preset(preset: u8) -> Graphics {
        let max = Graphics::MAXIMUM;
        match preset {
            0 => Graphics {
                preset: 0,
                shadows: 0,
                bounced_light: false,
                reflections: false,
                bloom: 0,
                render_scale: 67,
                upscaler: 1,
                dynamic: 0,
                antialias: 0,
                sharpness: 4,
                lod: 2,
                prepass: true,
            },
            1 => Graphics {
                preset: 1,
                shadows: 1,
                bounced_light: false,
                reflections: true,
                bloom: 1,
                render_scale: 85,
                upscaler: 1,
                dynamic: 0,
                antialias: 0,
                sharpness: 2,
                lod: 1,
                prepass: true,
            },
            2 => Graphics {
                preset: 2,
                shadows: 2,
                lod: 1,
                prepass: true,
                ..max
            },
            _ => max,
        }
    }

    /// The renderer's quality for these settings.
    pub(crate) fn quality(&self) -> velvet_render::Quality3D {
        velvet_render::Quality3D {
            shadow_samples: [0, 4, 8, 16][self.shadows.min(3) as usize],
            bounced_light: self.bounced_light,
            water_reflections: self.reflections,
            bloom_levels: [0, 3, 6][self.bloom.min(2) as usize],
            lod: self.lod.min(2) as u32,
            depth_prepass: self.prepass,
        }
    }

    /// The finishing passes for these settings.
    pub(crate) fn finishing(&self) -> velvet_render::Finishing3D {
        velvet_render::Finishing3D {
            upscale: if self.upscaler == 0 {
                velvet_render::Upscale3D::Bilinear
            } else {
                velvet_render::Upscale3D::Fsr1
            },
            antialias: if self.antialias == 1 {
                velvet_render::Antialias3D::Fxaa
            } else {
                velvet_render::Antialias3D::None
            },
            sharpness: self.sharpness.min(10) as f32 / 10.0,
        }
    }

    /// The frame time dynamic resolution keeps under (ms).
    pub(crate) fn budget_ms(&self) -> Option<f32> {
        match self.dynamic {
            1 => Some(1000.0 / 30.0),
            2 => Some(1000.0 / 60.0),
            _ => None,
        }
    }

    fn to_json(self) -> Value {
        serde_json::json!({
            "preset": self.preset, "shadows": self.shadows, "bounced_light": self.bounced_light,
            "reflections": self.reflections, "bloom": self.bloom, "render_scale": self.render_scale,
            "upscaler": self.upscaler, "dynamic": self.dynamic, "antialias": self.antialias,
            "sharpness": self.sharpness, "lod": self.lod, "prepass": self.prepass,
        })
    }

    fn from_json(map: &Map<String, Value>) -> Graphics {
        let mut g = Graphics::MAXIMUM;
        let n = |k: &str, d: u8| {
            map.get(k)
                .and_then(Value::as_u64)
                .map_or(d, |v| v.min(255) as u8)
        };
        let b = |k: &str, d: bool| map.get(k).and_then(Value::as_bool).unwrap_or(d);
        g.preset = n("preset", g.preset).min(PRESET_CUSTOM);
        g.shadows = n("shadows", g.shadows).min(3);
        g.bounced_light = b("bounced_light", g.bounced_light);
        g.reflections = b("reflections", g.reflections);
        g.bloom = n("bloom", g.bloom).min(2);
        let scale = n("render_scale", g.render_scale);
        g.render_scale = if RENDER_SCALES.contains(&scale) {
            scale
        } else {
            100
        };
        g.upscaler = n("upscaler", g.upscaler).min(1);
        g.dynamic = n("dynamic", g.dynamic).min(2);
        g.antialias = n("antialias", g.antialias).min(1);
        g.sharpness = n("sharpness", g.sharpness).min(10);
        g.lod = n("lod", g.lod).min(2);
        g.prepass = b("prepass", g.prepass);
        g
    }
}

static GRAPHICS: std::sync::Mutex<Graphics> = std::sync::Mutex::new(Graphics::MAXIMUM);

pub(crate) fn graphics() -> Graphics {
    *GRAPHICS.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn set_graphics(g: Graphics) {
    *GRAPHICS.lock().unwrap_or_else(|e| e.into_inner()) = g;
}

/// Where the settings live (the same file the language is kept in).
pub(crate) fn settings_path() -> PathBuf {
    // The tests never write the player's own settings.
    if cfg!(test) {
        return std::env::temp_dir()
            .join(format!("dq-test-settings-{}", std::process::id()))
            .join("settings.json");
    }
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SETTINGS") {
        return PathBuf::from(path);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("settings.json");
    }
    crate::exploration3d_state::source_root().join("data/settings.json")
}

fn read_file(path: &std::path::Path) -> Map<String, Value> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

/// Changes the settings file through `edit`, keeping every key it does not touch; written
/// whole to a temporary file first so a crash never leaves half a file.
pub(crate) fn update_file(edit: impl FnOnce(&mut Map<String, Value>)) -> std::io::Result<()> {
    update_file_at(&settings_path(), edit)
}

fn update_file_at(
    path: &std::path::Path,
    edit: impl FnOnce(&mut Map<String, Value>),
) -> std::io::Result<()> {
    let mut map = read_file(path);
    edit(&mut map);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&Value::Object(map))?)?;
    std::fs::rename(&temporary, path)
}

/// Reads the preferences from the settings file (missing ones keep their defaults).
pub(crate) fn load() {
    load_from(&settings_path());
}

fn load_from(path: &std::path::Path) {
    let map = read_file(path);
    let number = |key: &str| {
        map.get(key)
            .and_then(Value::as_u64)
            .map(|v| v.min(255) as u8)
    };
    if let Some(Value::Object(volumes)) = map.get("volumes") {
        for channel in Channel::ALL {
            if let Some(v) = volumes.get(channel.key()).and_then(Value::as_u64) {
                set_volume(channel, v.min(VOLUME_STEPS as u64) as u8);
            }
        }
    }
    if let Some(v) = number("text_speed") {
        set_text_speed(v);
    }
    if let Some(v) = number("mouse_sensitivity") {
        set_sensitivity(v);
    }
    if let Some(v) = map.get("fullscreen").and_then(Value::as_bool) {
        set_fullscreen(v);
    }
    if let Some(v) = map.get("native_resolution").and_then(Value::as_bool) {
        set_native_resolution(v);
    }
    if let Some(Value::Object(g)) = map.get("graphics") {
        set_graphics(Graphics::from_json(g));
    }
}

/// Writes the preferences into the settings file.
pub(crate) fn save() -> std::io::Result<()> {
    save_to(&settings_path())
}

fn save_to(path: &std::path::Path) -> std::io::Result<()> {
    update_file_at(path, |map| {
        let volumes: Map<String, Value> = Channel::ALL
            .iter()
            .map(|c| (c.key().to_string(), Value::from(volume(*c))))
            .collect();
        map.insert("volumes".into(), Value::Object(volumes));
        map.insert("text_speed".into(), Value::from(text_speed()));
        map.insert("mouse_sensitivity".into(), Value::from(sensitivity()));
        map.insert("fullscreen".into(), Value::from(fullscreen()));
        map.insert("native_resolution".into(), Value::from(native_resolution()));
        map.insert("graphics".into(), graphics().to_json());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_sliders_leave_the_mix_as_it_was_and_the_file_keeps_what_is_not_ours() {
        assert_eq!(gain(Channel::Music), 1.0);
        assert_eq!(look_scale(), 1.0);
        assert_eq!(text_rate(52.0), 52.0);
        set_volume(Channel::Music, 0);
        assert_eq!(gain(Channel::Music), 0.0);

        let dir = std::env::temp_dir().join(format!("dq-prefs-{}", std::process::id()));
        let path = dir.join("settings.json");
        let _ = std::fs::remove_dir_all(&dir);
        update_file_at(&path, |map| {
            map.insert("language".into(), Value::from("ja"));
        })
        .unwrap();
        set_volume(Channel::Music, 3);
        set_text_speed(2);
        set_sensitivity(8);
        save_to(&path).unwrap();
        set_volume(Channel::Music, VOLUME_STEPS);
        set_text_speed(1);
        set_sensitivity(SENSITIVITY_DEFAULT);
        load_from(&path);
        assert_eq!(
            (volume(Channel::Music), text_speed(), sensitivity()),
            (3, 2, 8)
        );
        assert_eq!(read_file(&path).get("language"), Some(&Value::from("ja")));
        set_volume(Channel::Music, VOLUME_STEPS);
        set_text_speed(1);
        set_sensitivity(SENSITIVITY_DEFAULT);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
