//! Lo que Chiquita comenta flota en la pantalla, en bocadillos que le salen de la boca (o de su
//! medallón, si su modelo no está): no es una línea del diálogo que haya que pasar, sino un
//! bocadillo que sale, se escribe mientras habla, sube y se apaga solo mientras lo demás sigue. La
//! dicen los guiones (cualquier línea suya), las escenas íntimas, la recepción, la alcoba y ella
//! misma cuando la tocas o la arrastras.
use super::*;

/// At most this many float at once (the oldest leaves first).
const MOST: usize = 3;

/// One of Chiquita's comments in the air.
pub(super) struct Float {
    text: String,
    /// Jealous and flustered (her other face).
    jealous: bool,
    /// How she says it (the balloon's shape): see `map_overlay::FloatLine::mood`.
    mood: u8,
    born: f32,
    life: f32,
    /// How long its words take to come out (as long as her mouth moves).
    reveal: f32,
}

/// How long her words take to come out (s): her mouth moves this long too.
pub(super) fn speaking_time(text: &str) -> f32 {
    (text.chars().count() as f32 * 0.048).clamp(0.5, 3.4)
}

/// The balloon for a cue: talking, cross, muttering, flustered, shouting, pleased.
fn mood_of(cue: mascot::Cue) -> u8 {
    match cue {
        mascot::Cue::Talk => 0,
        mascot::Cue::Jealous | mascot::Cue::Scold => 1,
        mascot::Cue::Away | mascot::Cue::Dazed => 2,
        mascot::Cue::Flustered => 3,
        mascot::Cue::Splat | mascot::Cue::Smash | mascot::Cue::Cover | mascot::Cue::Jab => 4,
        mascot::Cue::Cheer | mascot::Cue::Swoop => 5,
    }
}

impl App {
    /// Chiquita says `text` over whatever is on screen (talking, or jealous).
    pub(super) fn float_chiquita(&mut self, text: &str, jealous: bool) {
        let cue = if jealous {
            mascot::Cue::Jealous
        } else {
            mascot::Cue::Talk
        };
        self.float_chiquita_cue(text, cue);
    }

    /// Chiquita says `text`, doing `cue` as she says it.
    pub(super) fn float_chiquita_cue(&mut self, text: &str, cue: mascot::Cue) {
        self.float_chiquita_after(text, cue, 0.0);
    }

    /// Chiquita says `text` `delay` seconds from now, doing `cue` as she says it.
    pub(super) fn float_chiquita_after(&mut self, text: &str, cue: mascot::Cue, delay: f32) {
        let jealous = !matches!(
            cue,
            mascot::Cue::Talk | mascot::Cue::Cheer | mascot::Cue::Swoop
        );
        let text = text.trim();
        self.mascot_say(cue, text, delay);
        if text.is_empty() || self.floats.iter().any(|f| f.text == text) {
            return;
        }
        let reveal = speaking_time(text);
        // Long enough to read twice over once it is all out.
        let life = (reveal + 2.4 + text.chars().count() as f32 * 0.04).min(9.0);
        self.floats.push(Float {
            text: text.to_owned(),
            jealous,
            mood: mood_of(cue),
            born: self.elapsed + delay.max(0.0),
            life,
            reveal,
        });
        if self.floats.len() > MOST {
            self.floats.remove(0);
        }
    }

    /// Those whose time is up go.
    pub(super) fn tick_floats(&mut self) {
        let now = self.elapsed;
        self.floats.retain(|f| now - f.born < f.life);
    }

    /// The floats for the overlay, oldest first (those still to be said are not there yet).
    pub(super) fn floats_view(&self) -> Vec<map_overlay::FloatLine> {
        // On a guion's stage she speaks in the scene itself (her bubble beside her): no asides.
        if self.stage_up() {
            return Vec::new();
        }
        let face = |jealous: bool| {
            guion::portraits()
                .find("chiquita", jealous.then_some("celosa"))
                .map(|p| map_overlay::PortraitRef {
                    image: p.image,
                    crop: p.crop,
                })
        };
        self.floats
            .iter()
            .filter(|f| self.elapsed >= f.born)
            .map(|f| map_overlay::FloatLine {
                text: f.text.clone(),
                age_ms: ((self.elapsed - f.born).max(0.0) * 1000.0) as u32,
                life_ms: (f.life * 1000.0) as u32,
                portrait: face(f.jealous),
                jealous: f.jealous,
                mood: f.mood,
                reveal_ms: (f.reveal * 1000.0) as u32,
            })
            .collect()
    }
}
