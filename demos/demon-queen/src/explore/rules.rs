//! What the Queen may do where, and what Chiquita says about it: the brothel opens at night to a
//! hooded customer (the cloak from the shop) until Rosalía hands her the token; the hypnotist
//! will not talk to the Queen in her royal dress; the bikini stays at the springs until her lust
//! is awake enough. Each refusal comes with Chiquita's word and an orange «Nota» on what to do.
use super::mascot::Cue;
use super::ui_sfx::{self, UiSfx};
use super::*;

/// The shop's cloak (its item id) and the outfit it is.
pub(super) const CAPE_ITEM: &str = "capa_reina";
const CAPE_OUTFIT: &str = "capa";
const BIKINI_OUTFIT: &str = "bikini";
const GREEN_OUTFIT_SLOT: u8 = 1;
/// The lust rank from which the bikini may go out of the springs: «Despierta» (the one before
/// «Lasciva»), from 4 points.
const BIKINI_OUT_POINTS: i64 = 4;
/// Story flag: Chiquita has already said the bikini stays at the springs.
const BIKINI_OUT_NOTE: &str = "termas_bikini_aviso";

/// The wardrobe slot of an outfit of the catalog.
pub(super) fn outfit_slot(id: &str) -> Option<u8> {
    achievements::catalog()
        .outfits
        .iter()
        .find(|o| o.id == id)
        .map(|o| o.slot)
}

impl App {
    /// Night: the same hours the bed counts as night.
    pub(super) fn is_night(&self) -> bool {
        !(0.2..=0.8).contains(&self.environment_cycle.phase())
    }

    /// Her lust points, as the pause menu's status page counts them.
    pub(super) fn lust_points(&self) -> i64 {
        let n = |k: &str| i64::from(self.story.get(k).max(0));
        n(ero::MASTURBACIONES)
            + n(ero::ORGASMOS)
            + 2 * n(ero::ENCUENTROS)
            + n(ero::FUTA)
            + n(ero::HIPNOTIZADA)
    }

    fn wearing(&self, id: &str) -> bool {
        outfit_slot(id) == Some(self.outfit)
    }

    /// Chiquita says it, and the note says what to do.
    fn refuse(&mut self, chiquita: &str, cue: Cue, title: &str, note: &str) {
        self.float_chiquita_cue(chiquita, cue);
        self.notice_note(format!("{title}\n{note}"));
    }

    /// F at the brothel's door: in when Rosalía's token is hers; before that only at night and in
    /// the cloak (she puts it on at the door).
    pub(super) fn try_enter_brothel(&mut self) {
        let welcomed = self.story.get("burdel_bienvenida") > 0;
        if !welcomed {
            if !self.is_night() {
                self.refuse(
                    "—¿De día? Ni loca. Ese sitio abre de noche… y a plena luz te ve todo el reino.",
                    Cue::Scold,
                    "El burdel abre de noche",
                    "Vuelve cuando anochezca (duerme en tu cama del castillo para que pase el día).",
                );
                return;
            }
            if self.inventory.get(CAPE_ITEM).copied().unwrap_or(0) == 0 {
                self.refuse(
                    "—Ni loca entro ahí contigo así. ¡Que te conoce todo el reino! Al menos compra algo para cubrirte.",
                    Cue::Scold,
                    "Necesitas cubrirte",
                    "Compra la capa con capucha en la tienda de la villa (100 de oro) y vuelve de noche.",
                );
                return;
            }
            // In the cloak, hood up.
            if let Some(slot) = outfit_slot(CAPE_OUTFIT) {
                if self.outfit != slot && slot < wardrobe_outfit_count() {
                    self.outfit = slot;
                    self.wardrobe_selection = slot;
                    ui_sfx::play(UiSfx::Equip);
                }
            }
        }
        self.enter_brothel();
    }

    /// Something bought at the shop: the cloak is an outfit too, and Chiquita has her say.
    pub(super) fn on_bought(&mut self, id: &str) {
        if id != CAPE_ITEM {
            return;
        }
        self.achievements.grant_outfit(CAPE_OUTFIT);
        self.float_chiquita_cue(
            "—¿Esto es tu gran disfraz? Una capa. Con los cuernos asomando por la capucha. Todo el mundo sabrá que eres tú.",
            Cue::Jealous,
        );
        self.float_chiquita_after(
            "—…Bueno. De noche y con la cabeza gacha, a lo mejor cuela.",
            Cue::Talk,
            6.5,
        );
        self.notice_note(
            "Tienes la capa\nVe al burdel de noche: te la pondrás en la puerta.".to_owned(),
        );
        self.persist_3d_state(true);
    }

    /// F at the hypnotist: not in the royal dress (he stammers); green is informal enough.
    pub(super) fn mage_will_talk(&mut self) -> bool {
        if self.outfit == GREEN_OUTFIT_SLOT {
            return true;
        }
        hud::mage_says(
            "—¡M-M-Ma… Majes…! ¿La R-Reina? Y-yo no… no he hecho n-nada…",
            self.elapsed + 4.5,
        );
        self.float_chiquita_after(
            "—Intimidas mucho con el traje real. Ponte algo más informal si quieres que hable contigo.",
            Cue::Talk,
            1.2,
        );
        self.notice_note(
            "El mago se pone nervioso\nPonte el vestido verde (vestidor del castillo) para hablar con él."
                .to_owned(),
        );
        false
    }

    /// Whether the bikini may be worn out of the springs yet.
    pub(super) fn bikini_may_go_out(&self) -> bool {
        self.lust_points() >= BIKINI_OUT_POINTS
    }

    fn bikini_note(&mut self, chiquita: &str) {
        self.refuse(
            chiquita,
            Cue::Scold,
            "El bikini es para las termas",
            "Solo podrás llevarlo fuera cuando tu deseo llegue a «Despierta» (Ero status).",
        );
    }

    /// The springs' changing room: once the welcome has given her the bikini she wears it there,
    /// and what she came in waits for her way out.
    pub(super) fn change_into_bikini(&mut self) {
        let Some(slot) = outfit_slot(BIKINI_OUTFIT) else {
            return;
        };
        if self.outfit == slot
            || slot >= wardrobe_outfit_count()
            || !self
                .achievements
                .outfit_available(achievements::catalog(), slot)
        {
            return;
        }
        self.outfit_before_baths = Some(self.outfit);
        self.outfit = slot;
        self.wardrobe_selection = slot;
    }

    /// Out of the springs: her own clothes back from the changing room. In the bikini too soon,
    /// Chiquita says so the first time (and with nothing of hers waiting, the royal dress).
    pub(super) fn left_baths_in_bikini(&mut self) {
        let waiting = self.outfit_before_baths.take();
        if !self.wearing(BIKINI_OUTFIT) {
            return;
        }
        let too_soon = !self.bikini_may_go_out();
        let outfit = match waiting {
            Some(outfit) => outfit,
            None if too_soon => 0,
            None => return,
        };
        self.outfit = outfit;
        self.wardrobe_selection = outfit;
        if too_soon && (waiting.is_none() || self.story.get(BIKINI_OUT_NOTE) == 0) {
            self.story.values.insert(BIKINI_OUT_NOTE.to_owned(), 1);
            self.bikini_note(
                "—¿Salir así a la calle? Ni loca. Que te vea media villa en bikini… Vístete.",
            );
        }
        self.persist_3d_state(true);
    }

    /// The wardrobe's choice: the bikini only at the springs until her lust is awake.
    pub(super) fn wardrobe_allows(&mut self, slot: u8) -> bool {
        if outfit_slot(BIKINI_OUTFIT) == Some(slot)
            && self.baths.is_none()
            && !self.bikini_may_go_out()
        {
            self.bikini_note("—¿El bikini en el castillo? Ni hablar. Eso se queda en las termas.");
            return false;
        }
        true
    }
}
