//! Lo que el acto 1 pone en marcha por sí solo entre los guiones y el mundo: la noche de los
//! apagados (al despertar, cuando toca) y el combate con Varrak (su llegada antes, su despertar
//! después).
use super::*;

impl App {
    /// Each step: what the first act starts by itself once nothing else is on show.
    pub(super) fn tick_act_one(&mut self) {
        let free = self.talk.is_none()
            && self.stage.is_none()
            && self.alcove.is_none()
            && !self.bed_scene_open()
            && self.drink.is_none();
        if !free {
            return;
        }
        // The night of the dimmed: the bells wake her (`acto1.ataque_villa`).
        if self.story.get("ataque_pendiente") > 0 {
            self.story.values.insert("ataque_pendiente".to_owned(), 0);
            self.environment_cycle.set_phase(0.02);
            self.start_talk_at("acto1.ataque_villa");
            return;
        }
        // Varrak's arrival told: the fight.
        if self.pending_boss_fight {
            self.pending_boss_fight = false;
            self.start_boss_fight();
        }
    }

    /// After she has slept: the night the bells ring, once Gorou has looked up and Varrak has not
    /// been seen yet.
    pub(super) fn night_after_sleep(&mut self) {
        if self.story.get("gorou_cabeza_alta") > 0 && self.story.get("varrak_visto") == 0 {
            self.story.values.insert("ataque_pendiente".to_owned(), 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(
            PathBuf::new(),
            "act-one-test".into(),
            "bedroom_inside".into(),
            PathBuf::new(),
        )
    }

    #[test]
    fn sleeping_after_the_corral_rings_the_bells_once() {
        let mut app = app();
        // Before the corral, a night is only a night.
        app.sleep_in_bed();
        app.tick_act_one();
        assert!(app.talk.is_none());
        app.story.values.insert("gorou_cabeza_alta".into(), 1);
        app.sleep_in_bed();
        app.tick_act_one();
        let lib = guion::library();
        assert_eq!(
            app.talk.as_ref().map(|t| t.scene_id(lib)),
            Some("acto1.ataque_villa")
        );
        // Once Varrak has been seen, no more bells.
        app.talk = None;
        app.story.values.insert("varrak_visto".into(), 1);
        app.sleep_in_bed();
        app.tick_act_one();
        assert!(app.talk.is_none());
    }

    #[test]
    fn the_corral_errand_comes_from_lira_and_is_told_at_the_corral() {
        let mut app = app();
        app.story.values.insert("lamento_visto".into(), 1);
        // The corral says nothing until Lira has passed the errand on.
        assert!(!app.start_talk(None, Some("corral")));
        assert!(app.start_talk(Some("lira"), None));
        let lib = guion::library();
        assert_eq!(
            app.talk.as_ref().map(|t| t.scene_id(lib)),
            Some("acto1.corral_aviso")
        );
        app.talk = None;
        app.story.values.insert("corral_pedido".into(), 1);
        assert!(app.start_talk(None, Some("corral")));
        assert_eq!(
            app.talk.as_ref().map(|t| t.scene_id(lib)),
            Some("acto1.corral_grizna")
        );
    }
}
