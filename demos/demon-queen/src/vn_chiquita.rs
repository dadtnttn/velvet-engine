//! Chiquita viva en la novela visual (el prólogo y las escenas de guion): su modelo 3D, el mismo
//! que en el mundo (`characters/chiquita.glb`, el renderizador del overlay), en lugar de sus
//! retratos fijos. Cuando habla desde su rincón asoma por la esquina de abajo a la derecha, con su
//! bocadillo junto a la cabeza, y se esconde cuando habla otro; cuando está en escena (la lección
//! de la pizarra, sus `@sprite chiquita chiquita/pose_…`) está donde estaba su pose.
//!
//! No está quieta: aparece de un salto y se esconde, flota un poco (es un rescoldo, no pisa el
//! suelo), respira, mueve la boca mientras sale el texto, parpadea, cada frase nueva le da un
//! pequeño impulso y un giro de cabeza, y hace el gesto de lo que dice: regaña con el dedo,
//! se cruza de brazos enfurruñada, se pone colorada, se anima, o se da la vuelta. El gesto sale
//! del humor del guion (`**Chiquita (enfadada):**`) o, si no lo hay, de cómo suena la frase.
use std::time::Instant;

use crate::exploration3d::map_overlay::mascot::Mascot;
use crate::exploration3d::map_overlay::MascotView;
use crate::game::SceneView;

/// Her clips, as the renderer numbers them (`map_overlay::mascot::CLIPS`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Clip {
    Idle = 0,
    Talk = 1,
    Jealous = 2,
    Away = 3,
    Flustered = 4,
    Cheer = 5,
    Enter = 8,
    Scold = 9,
}

impl Clip {
    /// How long it lasts (s), and whether it loops.
    fn length(self) -> (f32, bool) {
        match self {
            Clip::Idle => (4.0, true),
            Clip::Talk => (2.0, true),
            Clip::Jealous => (2.4, true),
            Clip::Away => (3.0, true),
            Clip::Flustered => (1.6, true),
            Clip::Cheer => (1.2, true),
            Clip::Enter => (0.9, false),
            Clip::Scold => (1.2, true),
        }
    }

    /// How far her eyelids are down in it (sulking through half-shut eyes).
    fn lids(self) -> f32 {
        match self {
            Clip::Jealous => 0.4,
            Clip::Away => 0.3,
            Clip::Scold => 0.22,
            _ => 0.0,
        }
    }
}

/// Her gesture for a line: the guion's mood, else what the words sound like.
pub(crate) fn gesture(mood: Option<&str>, text: &str) -> Clip {
    if let Some(m) = mood {
        // The guion hands moods over as slugs (`de_espaldas`, `reganona`).
        let m = crate::exploration3d::guion::slug(m);
        let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
        return if has(&["enfad", "enoj", "regan", "furios", "mandon", "brava"]) {
            // A hand on her hip, wagging a finger.
            Clip::Scold
        } else if has(&[
            "celos", "enfurrun", "molest", "harta", "seca", "sarcas", "altiv", "orgullos", "digna",
        ]) {
            // Arms crossed, chin up, tapping her foot.
            Clip::Jealous
        } else if has(&[
            "avergon", "sonroj", "colorad", "nervios", "apurad", "incomod",
        ]) {
            // Hands on her cheeks, shaking her head.
            Clip::Flustered
        } else if has(&[
            "content", "animad", "feliz", "triunf", "burlon", "ufan", "alegr",
        ]) {
            // Jumping, arms up.
            Clip::Cheer
        } else if has(&["espaldas", "ofendid", "dolid"]) {
            // Her back to the scene, peeking.
            Clip::Away
        } else {
            Clip::Talk
        };
    }
    let t = text.trim_start_matches(['—', ' ', '«']);
    if t.starts_with('¡') || t.contains('!') {
        Clip::Scold
    } else if t.starts_with('…') {
        Clip::Flustered
    } else {
        Clip::Talk
    }
}

/// Her gesture and turn on stage from her pose's sprite (`chiquita/pose_regana`…).
fn pose_gesture(asset: &str) -> (Clip, f32) {
    let name = asset.rsplit('/').next().unwrap_or("");
    if name.contains("regana") {
        (Clip::Scold, 8.0)
    } else if name.contains("celosa") {
        (Clip::Jealous, 0.0)
    } else if name.contains("animada") {
        (Clip::Cheer, 6.0)
    } else if name.contains("avergonzada") {
        (Clip::Flustered, 10.0)
    } else {
        (Clip::Talk, 12.0)
    }
}

/// Where she is in the scene.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Place {
    /// Popping up at the bottom right (her own lines, with no box).
    Corner,
    /// On stage where her pose was: her feet on the screen, how tall, her turn (degrees).
    Stage {
        feet: [f32; 2],
        height: f32,
        yaw: f32,
    },
}

/// What the node on show wants of her.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Want {
    pub place: Place,
    /// The line is hers.
    pub speaking: bool,
    pub gesture: Clip,
    pub node: String,
}

impl Want {
    pub(crate) fn on_stage(&self) -> bool {
        matches!(self.place, Place::Stage { .. })
    }
}

/// Her corner: popping up at the bottom right, turned a little toward the middle of the screen.
const CORNER_FEET: [f32; 2] = [1738.0, 1082.0];
const CORNER_HEIGHT: f32 = 520.0;
const CORNER_YAW: f32 = -16.0;
/// Where her words hang beside her head in the corner: the bubble's right edge and its bottom,
/// and where its tail points (her face).
pub(crate) const BUBBLE_ANCHOR: [f32; 2] = [1604.0, 628.0];
pub(crate) const TAIL_TIP: [f32; 2] = [1662.0, 668.0];
/// Turned away from what she should not watch.
const AWAY_YAW: f32 = 112.0;

/// Rooms whose pictures show her already: the throne of her memory, in the prologue's dream. There
/// her words come in their bubble and she does not pop up beside herself.
const IN_THE_PICTURE: [&str; 1] = ["queen_memory"];

/// She is drawn in the node's picture itself.
pub(crate) fn in_the_picture(view: &SceneView) -> bool {
    IN_THE_PICTURE.contains(&view.room_id.as_str())
}

/// What a node shows of her: her sprite's place on stage, or her own line in her corner.
pub(crate) fn want(view: &SceneView) -> Option<Want> {
    let speaking = view.speaker_id == "chiquita";
    if let Some(layer) = view
        .layers
        .iter()
        .find(|l| l.asset.starts_with("chiquita/"))
    {
        let (pose, yaw) = pose_gesture(&layer.asset);
        // Her poses were cut from her 3D picture (feet at 960, 735; 600 px tall) at 670, 20
        // (580 × 1010): she stands at the same place on the sprite's slot.
        let k = layer.height as f32 / 1010.0;
        let feet = [layer.x as f32 + 290.0 * k, layer.y as f32 + 715.0 * k];
        // Her line's own mood, else her pose's; while someone else talks she listens (a sulk stays
        // a sulk).
        let gesture = match (speaking, &view.mood) {
            (true, Some(mood)) => gesture(Some(mood), &view.text),
            (true, None) => pose,
            (false, _) if pose == Clip::Jealous => pose,
            (false, _) => Clip::Idle,
        };
        return Some(Want {
            place: Place::Stage {
                feet,
                height: 600.0 * k,
                yaw,
            },
            speaking,
            gesture,
            node: view.id.clone(),
        });
    }
    (speaking && !in_the_picture(view)).then(|| Want {
        place: Place::Corner,
        speaking,
        gesture: gesture(view.mood.as_deref(), &view.text),
        node: view.id.clone(),
    })
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Out past the end and back (a pop): 0 → 1, overshooting a little.
fn back_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    let c = 1.6;
    1.0 + (c + 1.0) * x * x * x + c * x * x
}

/// Back a little before going: 0 → 1, dipping under 0 first.
fn back_in(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let c = 1.25;
    (c + 1.0) * x * x * x - c * x * x
}

/// A small random number stream (0..1).
fn roll(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed % 10_000) as f32 / 10_000.0
}

/// Her mouth while words come out, `t` seconds in: syllables about nine a second, each its own
/// size, now and then a pause between words.
fn flap(t: f32) -> f32 {
    let beat = 0.115;
    let n = (t / beat).floor();
    let f = t / beat - n;
    let h = ((n * 12.9898 + 4.1).sin() * 43_758.547).fract().abs();
    let size = if h < 0.16 { 0.04 } else { 0.32 + 0.68 * h };
    size * (f * std::f32::consts::PI).sin().max(0.0).powf(0.8)
}

/// Chiquita alive over the visual novel: her renderer (made the first time she shows) and what she
/// is doing.
pub(crate) struct Live {
    renderer: Option<Mascot>,
    tried: bool,
    /// Her model is there to draw (else the VN keeps her pictures).
    available: bool,
    clock: Instant,
    last: f32,
    /// 0 out of sight, 1 all the way up; on her way up; when she last sprang up.
    out: f32,
    rising: bool,
    entered: f32,
    place: Place,
    feet: [f32; 2],
    height: f32,
    /// Her turn (degrees) and how fast it goes: a spring, so she whips round and settles.
    yaw: f32,
    yaw_vel: f32,
    /// Each new line turns her head a touch the other way.
    side: f32,
    clip: Clip,
    clip_at: f32,
    prev: Clip,
    prev_at: f32,
    switched: f32,
    gesture: Clip,
    node: String,
    talk_from: f32,
    talk_until: f32,
    /// When the line on show began (a little pop).
    beat: f32,
    blinks: [f32; 2],
    next_blink: f32,
    seed: u32,
    /// A clock set by hand (captures step her through time), else the real one.
    manual: Option<f32>,
}

impl Live {
    pub(crate) fn new() -> Self {
        let available = crate::exploration3d_state::data_dir()
            .join("assets/3d/characters/chiquita.glb")
            .exists();
        Self {
            renderer: None,
            tried: false,
            available,
            clock: Instant::now(),
            last: 0.0,
            out: 0.0,
            rising: false,
            entered: -10.0,
            place: Place::Corner,
            feet: CORNER_FEET,
            height: CORNER_HEIGHT,
            yaw: CORNER_YAW,
            yaw_vel: 0.0,
            side: 1.0,
            clip: Clip::Idle,
            clip_at: 0.0,
            prev: Clip::Idle,
            prev_at: 0.0,
            switched: -10.0,
            gesture: Clip::Talk,
            node: String::new(),
            talk_from: -10.0,
            talk_until: -10.0,
            beat: -10.0,
            blinks: [-10.0, -10.0],
            next_blink: 1.2,
            seed: 0x9e37_79b9,
            manual: None,
        }
    }

    /// She can be drawn alive (her model is there and drawing her has not failed).
    pub(crate) fn ready(&self) -> bool {
        self.available && (!self.tried || self.renderer.is_some())
    }

    /// She is on screen or on her way in or out (the VN keeps drawing frames).
    pub(crate) fn animating(&self) -> bool {
        self.ready() && (self.out > 0.0 || self.rising)
    }

    fn now(&self) -> f32 {
        self.manual
            .unwrap_or_else(|| self.clock.elapsed().as_secs_f32())
    }

    /// Steps her `seconds` through `want` on a clock of her own, a frame at a time (a capture of the
    /// visual novel shows her as she would be by then).
    pub(crate) fn run_for(&mut self, want: Option<Want>, typing_for: f32, seconds: f32) {
        let mut t = self.manual.unwrap_or(0.0);
        let end = t + seconds;
        while t < end {
            t = (t + 1.0 / 60.0).min(end);
            self.manual = Some(t);
            self.update(want.clone(), t < typing_for);
        }
    }

    /// One frame: what the node wants of her, and whether its line is still typing out.
    pub(crate) fn update(&mut self, want: Option<Want>, typing: bool) {
        let now = self.now();
        let dt = (now - self.last).clamp(0.0, 0.1);
        self.last = now;
        match want {
            Some(w) => {
                let corner = matches!(w.place, Place::Corner);
                let was_corner = matches!(self.place, Place::Corner);
                if self.out <= 0.0 || corner != was_corner {
                    // Where she is now: up from the corner, or on stage at once.
                    let (feet, height, yaw) = target(w.place);
                    self.feet = feet;
                    self.height = height;
                    self.yaw = yaw + if corner { 25.0 } else { -8.0 };
                    self.yaw_vel = 0.0;
                    self.entered = now;
                    if !corner {
                        self.out = self.out.min(0.15);
                    }
                }
                if w.node != self.node {
                    // A new line: a little pop, her head turning a touch the other way.
                    self.node = w.node.clone();
                    self.beat = now;
                    self.side = -self.side;
                    self.yaw_vel += 55.0 * self.side;
                    if w.speaking {
                        self.talk_from = now;
                        self.talk_until = f32::INFINITY;
                    } else {
                        self.talk_until = self.talk_until.min(now);
                    }
                }
                if self.talk_until.is_infinite() && !typing {
                    self.talk_until = now + 0.18;
                }
                self.place = w.place;
                self.gesture = w.gesture;
                self.rising = true;
            }
            None => {
                self.rising = false;
                self.node.clear();
                self.talk_until = self.talk_until.min(now);
            }
        }
        // Up or down: she springs up into view and ducks back down.
        let rate = if self.rising { dt / 0.32 } else { -dt / 0.36 };
        self.out = (self.out + rate).clamp(0.0, 1.0);
        // Where she stands, eased there; her turn a spring.
        let (feet, height, yaw) = target(self.place);
        let k = (dt * 6.0).min(1.0);
        self.feet = [
            self.feet[0] + (feet[0] - self.feet[0]) * k,
            self.feet[1] + (feet[1] - self.feet[1]) * k,
        ];
        self.height += (height - self.height) * k;
        let aim = if self.gesture == Clip::Away {
            AWAY_YAW
        } else {
            yaw
        };
        let stiff = 150.0f32;
        let damp = 2.0 * 0.55 * stiff.sqrt();
        let steps = ((dt / (1.0 / 120.0)).ceil() as usize).clamp(1, 12);
        let h = dt / steps as f32;
        for _ in 0..steps {
            self.yaw_vel += ((aim - self.yaw) * stiff - self.yaw_vel * damp) * h;
            self.yaw += self.yaw_vel * h;
        }
        // Blinks: every few seconds, now and then twice.
        if now >= self.next_blink {
            let twice = roll(&mut self.seed) < 0.22;
            self.blinks = [now, if twice { now + 0.26 } else { -10.0 }];
            self.next_blink = now + 1.8 + 3.2 * roll(&mut self.seed);
        }
        // Her clip: popping up, else her gesture (talking while her words come out).
        let talking = now >= self.talk_from && now < self.talk_until;
        let entering =
            matches!(self.place, Place::Corner) && self.rising && now - self.entered < 0.9;
        let wanted = if entering {
            Clip::Enter
        } else {
            match self.gesture {
                Clip::Talk | Clip::Idle if talking => Clip::Talk,
                Clip::Talk | Clip::Idle => Clip::Idle,
                other => other,
            }
        };
        if wanted != self.clip {
            self.prev = self.clip;
            self.prev_at = self.clip_at;
            self.clip = wanted;
            self.clip_at = now;
            self.switched = now;
        }
    }

    /// How she is this frame, for her renderer (`None` while she is out of sight).
    pub(crate) fn view(&self) -> Option<MascotView> {
        if !self.ready() || (self.out <= 0.0 && !self.rising) {
            return None;
        }
        let now = self.now();
        let pi = std::f32::consts::PI;
        let corner = matches!(self.place, Place::Corner);
        let up = if self.rising {
            back_out(self.out)
        } else {
            1.0 - back_in(1.0 - self.out)
        };
        let mut feet = self.feet;
        let height = self.height;
        let mut alpha = 1.0f32;
        let mut squash = [1.0f32, 1.0, 1.0];
        if corner {
            // Up from below the screen's edge, stretched while she moves.
            feet[1] += (1.0 - up) * (height + 60.0);
            if self.out > 0.0 && self.out < 1.0 {
                let s = 0.09 * (self.out * pi).sin();
                squash = [1.0 - s * 0.5, 1.0 + s, 1.0 - s * 0.5];
            }
        } else {
            // On stage she fades in with a little hop.
            alpha = ease(self.out / 0.6);
            feet[1] -= (self.out.min(1.0) * pi).sin() * 26.0 * (1.0 - self.out);
        }
        // She floats (a rescoldo does not touch the ground), breathing.
        let hover = ((now * 1.6).sin() * 0.5 + 0.5) * 9.0 + 4.0;
        feet[1] -= hover * up.clamp(0.0, 1.0);
        let mut roll = (now * 1.15).sin() * 1.4 + (now * 0.53).sin() * 0.8;
        let talking = now >= self.talk_from && now < self.talk_until;
        let pitch = if talking {
            (now * 3.3).sin() * 1.6 + 1.2
        } else {
            (now * 0.9).sin() * 0.6
        };
        // A new line: a quick stretch up and settle.
        let b = (-((now - self.beat - 0.06) / 0.09).powi(2)).exp();
        if b > 0.002 {
            squash = [
                squash[0] * (1.0 - 0.035 * b),
                squash[1] * (1.0 + 0.06 * b),
                squash[2] * (1.0 - 0.035 * b),
            ];
        }
        if self.clip == Clip::Scold {
            roll += (now * 6.0).sin() * 1.2;
        }
        let blink = {
            let mut shut = 0.0f32;
            for at in self.blinks {
                let t = now - at;
                let b = if !(0.0..0.2).contains(&t) {
                    0.0
                } else if t < 0.06 {
                    t / 0.06
                } else if t < 0.1 {
                    1.0
                } else {
                    1.0 - (t - 0.1) / 0.1
                };
                shut = shut.max(b);
            }
            let k = ((now - self.switched) / 0.22).clamp(0.0, 1.0);
            shut.max(self.clip.lids() * k + self.prev.lids() * (1.0 - k))
        };
        let mouth = if talking {
            flap(now - self.talk_from) * if self.clip == Clip::Jealous { 0.6 } else { 1.0 }
        } else {
            match self.clip {
                Clip::Cheer => 0.45 + 0.2 * (now * 8.0).sin().abs(),
                Clip::Flustered => 0.14 + 0.1 * (now * 11.0).sin(),
                Clip::Enter if now - self.entered < 0.6 => 0.45,
                _ => 0.0,
            }
        };
        let clip_time = |clip: Clip, at: f32| {
            let (length, looping) = clip.length();
            let t = (now - at).max(0.0);
            let t = if looping { t } else { t.min(length - 0.01) };
            (t * 1000.0) as u32
        };
        Some(MascotView {
            clip: self.clip as u8,
            time_ms: clip_time(self.clip, self.clip_at),
            prev_clip: self.prev as u8,
            prev_time_ms: clip_time(self.prev, self.prev_at),
            blend: (((now - self.switched) / 0.22).clamp(0.0, 1.0) * 1000.0) as u16,
            feet: [feet[0].round() as i32, feet[1].round() as i32],
            height: height.round().max(1.0) as u16,
            yaw: (self.yaw * 10.0) as i16,
            pitch: (pitch * 10.0) as i16,
            roll: (roll * 10.0) as i16,
            squash: squash.map(|k| (k * 1000.0) as u16),
            alpha: (alpha.clamp(0.0, 1.0) * 1000.0) as u16,
            crack: None,
            clock_ms: (now * 1000.0) as u32,
            blink: (blink.clamp(0.0, 1.0) * 1000.0) as u16,
            mouth: (mouth.clamp(0.0, 1.0) * 1000.0) as u16,
            dazed: 0,
        })
    }

    /// Her renderer, made ahead of her first appearance (loading her model takes a moment).
    pub(crate) fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.available || self.tried {
            return;
        }
        self.tried = true;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vn-chiquita-picture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        self.renderer = Mascot::new(device, queue, &layout);
        if self.renderer.is_none() {
            eprintln!("[vn] Chiquita alive is not available: her pictures stay");
        }
    }

    /// Her picture this frame and how much of it shows (`None` while she is out of sight).
    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(&wgpu::TextureView, f32)> {
        let view = self.view()?;
        self.prepare(device, queue);
        let renderer = self.renderer.as_mut()?;
        renderer.render(device, queue, &view);
        Some((renderer.picture(), view.alpha.min(1000) as f32 / 1000.0))
    }

    /// Her last picture read back (sRGB bytes, premultiplied): for captures.
    pub(crate) fn snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<u8>)> {
        self.renderer.as_ref()?.snapshot(device, queue)
    }
}

/// Where a place puts her: her feet, how tall, her turn.
fn target(place: Place) -> ([f32; 2], f32, f32) {
    match place {
        Place::Corner => (CORNER_FEET, CORNER_HEIGHT, CORNER_YAW),
        Place::Stage { feet, height, yaw } => (feet, height, yaw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn her_gesture_comes_from_the_mood_or_the_words() {
        assert_eq!(gesture(Some("enfadada"), "—No."), Clip::Scold);
        assert_eq!(gesture(Some("celosa"), "—Te odio."), Clip::Jealous);
        assert_eq!(gesture(Some("avergonzada"), "—Ah. Eso."), Clip::Flustered);
        assert_eq!(gesture(Some("contenta"), "—Bien."), Clip::Cheer);
        assert_eq!(
            gesture(Some("de_espaldas"), "—Yo me voy a mirar la pared."),
            Clip::Away
        );
        assert_eq!(gesture(Some("orgullosa"), "—Soy yo."), Clip::Jealous);
        assert_eq!(
            gesture(Some("burlona"), "—Técnicamente, te muerdo."),
            Clip::Cheer
        );
        assert_eq!(gesture(None, "—¡Silencio!"), Clip::Scold);
        assert_eq!(gesture(None, "—…Les ha gustado."), Clip::Flustered);
        assert_eq!(gesture(None, "—¿De qué murió?"), Clip::Talk);
    }

    #[test]
    fn she_pops_up_talks_and_ducks_away() {
        let mut live = Live::new();
        live.available = true;
        let line = Want {
            place: Place::Corner,
            speaking: true,
            gesture: Clip::Talk,
            node: "a".into(),
        };
        for _ in 0..40 {
            live.update(Some(line.clone()), true);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(live.out > 0.9, "she is up");
        let v = live.view().expect("shown");
        assert!(v.height > 400);
        // Done talking and someone else speaks: she goes.
        for _ in 0..60 {
            live.update(None, false);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(live.view().is_none());
        assert!(!live.animating());
    }
}
