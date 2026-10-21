//! The war and the rest of the game as one:
//!
//! - **One purse.** The realm's treasury in the campaign is the Queen's own gold (the shop's, the
//!   castle works', the war's): it goes into the campaign when the war opens and comes back out
//!   every step of it.
//! - **The story reaches the campaign** (each thing once, `Campaign::story_marks`): Varrak woken in
//!   the story rides with her; the first Council's choices leave their mark on the north and the
//!   larder; the river stays closed to her armies until the first act ends («primero el norte»).
//! - **The campaign reaches the story**: every region she takes sets `guerra_<id>` and plays its
//!   scene; victories, the end, the relics and the powers are story variables too.
//! - **The war's own scenes** (`data/guiones/guerra.md`) play in the dialogue box over the war
//!   table, the campaign or a battle, one after another when the war is between moves.
//! - **The relics** (`data/guerra/reliquias.json`): found taking a region, or picked up on its
//!   battlefield; they fill the treasury, unlock war powers and pull threads of the story.
//! - **The war powers** (`war/src/powers.rs`): what the story and the relics give her to cast in a
//!   battle.
//! - **The second act over the map** (`act_two_beats`): Varrak in his suit at the table, the first
//!   Alliance line (hate and calm), Merin across the river, Medianoche found in a bank of the
//!   Ribera and Zillah's account of it, the price of the spark, Nemi's guard, a night at the table;
//!   each once, when the campaign has come that far.
use serde::Deserialize;

use super::*;
use demon_queen_war as wr;
use wr::data::{Faction, PROVINCES};
use wr::powers::Power;

/// The regions across the great river (the Alliance's land at the start): out of her armies'
/// reach until the first act ends.
/// The day's flag for the campaign's turn (one a day; cleared when she sleeps with the court's).
pub(crate) const TURN_TODAY: &str = "guerra_turno_hoy";

pub(crate) const EAST: [&str; 10] = [
    "marca",
    "cabeza",
    "trigales",
    "molinoalto",
    "ribera",
    "valdesanto",
    "monasterio",
    "puerto_santo",
    "capital",
    "minas",
];

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Reward {
    #[serde(default)]
    pub oro: i32,
    #[serde(default)]
    pub chispa: i32,
    #[serde(default)]
    pub comida: i32,
    #[serde(default)]
    pub poder: Option<String>,
    #[serde(default)]
    pub var: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Relic {
    pub id: String,
    pub nombre: String,
    pub region: String,
    /// «batalla»: on its region's battlefield; «region»: found taking the region.
    pub modo: String,
    pub texto: String,
    #[serde(default)]
    pub recompensa: Reward,
}

impl Relic {
    pub fn on_field(&self) -> bool {
        self.modo == "batalla"
    }

    /// What it gives, in words.
    pub fn reward_text(&self) -> String {
        let r = &self.recompensa;
        let mut parts = Vec::new();
        if let Some(p) = r.poder.as_deref().and_then(power_of) {
            parts.push(format!("Poder: {}", p.spec().name));
        }
        if r.oro > 0 {
            parts.push(format!("+{} oro", r.oro));
        }
        if r.chispa > 0 {
            parts.push(format!("+{} chispa", r.chispa));
        }
        if r.comida > 0 {
            parts.push(format!("+{} comida", r.comida));
        }
        if r.var.is_some() {
            parts.push("Una pista".into());
        }
        parts.join(" · ")
    }
}

#[derive(Deserialize)]
struct RelicFile {
    reliquias: Vec<Relic>,
}

/// The war's relics (read once).
pub(crate) fn relics() -> &'static [Relic] {
    static R: std::sync::OnceLock<Vec<Relic>> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        let path = exploration3d_state::data_dir()
            .join("guerra")
            .join("reliquias.json");
        match std::fs::read(&path)
            .map_err(anyhow::Error::from)
            .and_then(|b| serde_json::from_slice::<RelicFile>(&b).map_err(anyhow::Error::from))
        {
            Ok(f) => f.reliquias,
            Err(e) => {
                eprintln!("[guerra] {}: {e:#}", path.display());
                Vec::new()
            }
        }
    })
}

pub(crate) fn power_of(id: &str) -> Option<Power> {
    Some(match id {
        "grito" => Power::Grito,
        "rugido" => Power::Rugido,
        "bendicion" => Power::Bendicion,
        "brasas" => Power::Brasas,
        "niebla" => Power::Niebla,
        "raices" => Power::Raices,
        _ => return None,
    })
}

pub(crate) fn power_id(p: Power) -> &'static str {
    match p {
        Power::Grito => "grito",
        Power::Rugido => "rugido",
        Power::Bendicion => "bendicion",
        Power::Brasas => "brasas",
        Power::Niebla => "niebla",
        Power::Raices => "raices",
    }
}

/// A relic lying on the battlefield, until a regiment of hers walks over it.
#[derive(Clone, Debug)]
pub(crate) struct FieldRelic {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub z: f32,
    pub taken: bool,
}

/// Where a battle's relic lies: between the two armies, off to one side, on open ground.
pub(crate) fn relic_spot(b: &wr::battle::Battle) -> (f32, f32) {
    let mid = |r: wr::maps::Rect| ((r.x0 + r.x1) / 2.0, (r.z0 + r.z1) / 2.0);
    let ((px, pz), (ex, ez)) = (mid(b.zone), mid(b.enemy_zone));
    let (dx, dz) = (ex - px, ez - pz);
    let l = dx.hypot(dz).max(1.0);
    let (mx, mz) = (px + dx * 0.45, pz + dz * 0.45);
    let side = if (b.setup.seed % 2) == 0 { 1.0 } else { -1.0 };
    let (x, z) = (mx - dz / l * 38.0 * side, mz + dx / l * 38.0 * side);
    let p = b.sim.field.nearest_free(x, z);
    (p.x, p.z)
}

impl App {
    // ------------------------------------------------------------------ one purse
    /// The Queen's gold into the campaign's treasury (the war opens, the campaign begins).
    pub(super) fn treasury_in(&mut self) {
        let gold = self.gold as i32;
        if let Some(s) = self.war_campaign.as_mut() {
            s.purse_mut(Faction::Reina).gold = gold;
        }
    }

    /// The treasury back into her purse (every step of the war: what it earns and spends is hers).
    pub(super) fn treasury_out(&mut self) {
        if let Some(s) = self.war_campaign.as_ref() {
            self.gold = s.purse(Faction::Reina).gold.max(0) as u32;
        }
    }

    /// Gold, spark and food given by the war (a relic): into the treasury when there is a campaign,
    /// the gold into her purse anyway.
    fn war_reward(&mut self, gold: i32, spark: i32, food: i32) {
        match self.war_campaign.as_mut() {
            Some(s) => {
                let f = s.purse_mut(Faction::Reina);
                f.gold += gold;
                f.spark += spark;
                f.food += food;
                self.treasury_out();
            }
            None => self.gold = (self.gold as i32 + gold).max(0) as u32,
        }
    }

    // ------------------------------------------------------------------ the story ↔ the campaign
    /// Whether her armies can go to `prov` as the story stands (the river waits for the first
    /// act's end).
    pub(super) fn river_closed(&self, prov: &str) -> bool {
        EAST.contains(&prov) && !self.varrak_awake()
    }

    /// What the story has done reaches the campaign, once each; the scenes it asks for are queued.
    pub(super) fn story_into_campaign(&mut self) {
        let varrak = self.varrak_awake();
        let story = self.story.clone();
        let mut scenes = Vec::new();
        let Some(s) = self.war_campaign.as_mut() else {
            return;
        };
        if varrak && !s.varrak_awake && s.story_mark("varrak") {
            s.varrak_joins();
            scenes.push("guerra.varrak_se_une");
        } else if varrak {
            s.story_mark("varrak");
        }
        if story.get("consejo_1_hecho") > 0 && s.story_mark("consejo_1") {
            use wr::rules::LogKind;
            if story.get("apagados_caza") > 0 {
                if let Some(a) = s.armies.iter_mut().find(|a| {
                    a.faction == Faction::Rebeldes && a.at == "cubil" && a.units.len() > 1
                }) {
                    a.units.pop();
                }
                s.note(
                    "Las cacerías que ordenó la Reina han dejado el Cubil con menos lobos.",
                    LogKind::Good,
                );
            }
            if story.get("apagados_encierro") > 0 {
                s.purse_mut(Faction::Reina).spark += 2;
                s.note(
                    "Los apagados encerrados siguen siendo de la Reina: su chispa no se pierde (+2 chispa).",
                    LogKind::Spark,
                );
            }
            if story.get("apagados_echar") > 0 {
                let a = s.purse_mut(Faction::Alianza);
                a.gold = (a.gold - 60).max(0);
                s.note(
                    "Los apagados que la Reina echó cruzan el río: la Alianza gasta oro en contenerlos.",
                    LogKind::Info,
                );
            }
            if story.get("rebeldes_hablar") > 0 {
                let r = s.purse_mut(Faction::Rebeldes);
                r.gold = (r.gold - 60).max(0);
                s.note(
                    "Los clanes del norte dudan: la Reina mandó a hablar con ellos, no a cobrar.",
                    LogKind::Info,
                );
            }
            if story.get("rebeldes_fuerza") > 0 {
                if let Some(q) = s.armies.iter_mut().find(|a| a.id == "a-reina") {
                    if q.units.len() < 8 {
                        q.units.push(wr::rules::new_unit("orcos", None));
                    }
                }
                s.note(
                    "La Reina exigió el tributo por la fuerza: una banda de orcos más marcha con ella.",
                    LogKind::Good,
                );
            }
            if story.get("rebeldes_esperar") > 0 || story.get("raciones_lira") > 0 {
                s.purse_mut(Faction::Reina).food += 4;
                s.note(
                    "Las despensas que ordenó Lira dan para más: +4 comida.",
                    LogKind::Good,
                );
            }
            if story.get("raciones_igual") > 0 {
                s.purse_mut(Faction::Reina).spark += 2;
                s.note(
                    "Los pueblos recuerdan que la Reina comió lo mismo que ellos (+2 chispa).",
                    LogKind::Spark,
                );
            }
        }
        for id in scenes {
            self.queue_war_scene(id);
        }
        // the powers the story has given since the war last looked
        for p in [Power::Grito, Power::Rugido, Power::Bendicion] {
            let seen = format!("poder_visto_{}", power_id(p));
            if self.war_powers().contains(&p) && self.story.get(&seen) == 0 {
                self.story.values.insert(seen, 1);
                self.queue_war_scene(&format!("guerra.poder_{}", power_id(p)));
            }
        }
    }

    /// The powers she has now: the story's and the relics'.
    pub(crate) fn war_powers(&self) -> Vec<Power> {
        let s = &self.story;
        let mut out = Vec::new();
        if s.get("gorou_cabeza_alta") > 0 || s.get("poder_grito") > 0 {
            out.push(Power::Grito);
        }
        if self.varrak_awake() || s.get("poder_rugido") > 0 {
            out.push(Power::Rugido);
        }
        if s.get("afecto_nyxara") >= 3
            || s.get("medic_relation") >= 3
            || s.get("poder_bendicion") > 0
        {
            out.push(Power::Bendicion);
        }
        for p in [Power::Brasas, Power::Niebla, Power::Raices] {
            if s.get(&format!("poder_{}", power_id(p))) > 0 {
                out.push(p);
            }
        }
        out
    }

    /// The campaign's news into the story: the regions she has taken since the last look (their
    /// variable, their scene, the relic found there), her victory or her fall.
    pub(super) fn campaign_into_story(&mut self) {
        let Some(s) = self.war_campaign.as_mut() else {
            return;
        };
        let owners: Vec<Faction> = s.provinces.iter().map(|p| p.owner).collect();
        let winner = s.winner;
        if s.story_owners.len() != owners.len() {
            s.story_owners = owners;
            return;
        }
        let mut taken = Vec::new();
        for (i, (&was, &now)) in s.story_owners.iter().zip(&owners).enumerate() {
            if was != Faction::Reina && now == Faction::Reina {
                taken.push(PROVINCES[i].id);
            }
        }
        s.story_owners = owners;
        // the first land across the river: an Alliance scout is brought to her
        if taken.iter().any(|id| EAST.contains(id)) {
            self.queue_war_scene("guerra.merin");
        }
        for id in taken {
            let var = format!("guerra_{id}");
            let first = self.story.get(&var) == 0;
            self.story.values.insert(var, 1);
            if first {
                self.queue_war_scene(&format!("guerra.toma_{id}"));
            }
            let found: Vec<String> = relics()
                .iter()
                .filter(|r| r.region == id && !r.on_field())
                .map(|r| r.id.clone())
                .collect();
            for r in found {
                self.grant_relic(&r);
            }
        }
        // (a scene seen is not queued again)
        match winner {
            Some(Faction::Reina) => self.queue_war_scene("guerra.victoria_final"),
            Some(_) => self.queue_war_scene("guerra.castillo_caido"),
            None => {}
        }
        self.act_two_beats();
    }

    /// The second act's beats that wait on how far the campaign has come (each scene once), and
    /// what their choices change in it.
    fn act_two_beats(&mut self) {
        let Some(s) = self.war_campaign.as_mut() else {
            return;
        };
        let (turn, joined) = (s.turn, s.varrak_awake);
        // Nemi's guard marches: a band more with the Queen's army
        if self.story.get("nemi_guardia") > 0 && s.story_mark("guardia") {
            if let Some(q) = s.armies.iter_mut().find(|a| a.id == "a-reina") {
                if q.units.len() < wr::data::MAX_UNITS {
                    q.units.push(wr::rules::new_unit("orcos", None));
                }
            }
            s.note(
                "La guardia de la Reina marcha con ella: una doncella, un orco y los que entrenan con ellos.",
                wr::rules::LogKind::Good,
            );
        }
        // what the second act's choices change in the campaign (each once)
        let story = &self.story;
        if story.get("merin_mensajera") > 0 && s.story_mark("merin_tregua") {
            s.truce_until = s.turn + 3;
            s.note(
                "El Consejo de la Alianza lee el mensaje que trajo Merin y lo debate: tres turnos sin cruzar el río.",
                wr::rules::LogKind::Lore,
            );
        }
        if story.get("merin_celda") > 0 && s.story_mark("merin_rabia") {
            s.purse_mut(Faction::Alianza).gold += 40;
            s.note(
                "La Alianza sabe que su exploradora está en una celda de la Reina: más oro para la guerra.",
                wr::rules::LogKind::Bad,
            );
        }
        if story.get("zillah_perdonada") > 0 && s.story_mark("zillah_despensa") {
            s.purse_mut(Faction::Reina).food += 8;
            s.note(
                "Zillah reordena la despensa con las cuentas de la regencia: +8 comida.",
                wr::rules::LogKind::Good,
            );
        }
        if story.get("nido_subir") > 0 && s.story_mark("nido_subio") && s.win_over("nido") {
            s.note(
                "La Reina subió sola al Nido. La miraron a la cara, y los clanes bajan con ella.",
                wr::rules::LogKind::Spark,
            );
        }
        // her army beside the Nest while it is the rebels': they ask for her, alone
        let nest_calls = s.owner("nido") == Faction::Rebeldes
            && s.queen_army()
                .is_some_and(|q| wr::data::adjacent(&q.at).contains(&"nido"));
        let suit = self.story.seen("guerra.consejo_traje");
        let wins = self.story.get("guerra_victorias");
        let mut beats = Vec::new();
        if joined && turn >= 3 {
            beats.push("guerra.consejo_traje");
        }
        if suit && joined && wins >= 2 {
            beats.push("guerra.guardia");
        }
        if suit && turn >= 8 {
            beats.push("guerra.noche_mesa");
        }
        if nest_calls {
            beats.push("guerra.nido_llamada");
        }
        for id in beats {
            self.queue_war_scene(id);
        }
    }

    /// A relic is hers: its reward, its variable, its words on the war's log and its scene.
    pub(super) fn grant_relic(&mut self, id: &str) -> bool {
        let Some(r) = relics().iter().find(|r| r.id == id).cloned() else {
            return false;
        };
        let var = format!("reliquia_{id}");
        if self.story.get(&var) > 0 {
            return false;
        }
        self.story.values.insert(var, 1);
        let rw = r.recompensa.clone();
        self.war_reward(rw.oro, rw.chispa, rw.comida);
        if let Some(p) = rw.poder.as_deref().and_then(power_of) {
            self.story
                .values
                .insert(format!("poder_{}", power_id(p)), 1);
            if let Some(powers) = self
                .war
                .as_mut()
                .and_then(|w| w.battle.as_mut())
                .and_then(|b| b.b.sim.powers.as_mut())
            {
                if !powers.has(p) {
                    powers.unlocked.push(p);
                }
            }
        }
        if let Some(v) = rw.var.as_deref() {
            self.story.values.insert(v.to_owned(), 1);
        }
        if let Some(s) = self.war_campaign.as_mut() {
            s.note(
                format!("Reliquia: {}. {}", r.nombre, r.reward_text()),
                wr::rules::LogKind::Lore,
            );
        }
        if let Some(w) = self.war.as_mut() {
            if let Some(c) = w.camp.as_mut() {
                c.toast = Some((format!("Reliquia: {}", r.nombre), r.reward_text(), 0.0));
            }
            if let Some(b) = w.battle.as_mut() {
                b.messages.push_front((
                    format!("Reliquia: {} · {}", r.nombre, r.reward_text()),
                    0xf2b54a,
                    0.0,
                ));
            }
        }
        ui_sfx::play(ui_sfx::UiSfx::Deal);
        self.queue_war_scene(&format!("guerra.reliquia_{id}"));
        eprintln!("[guerra] relic «{id}»");
        true
    }

    /// Her armies cannot cross yet: the scene the first time, a word after.
    pub(super) fn river_blocked(&mut self) {
        ui_sfx::play(ui_sfx::UiSfx::Error);
        if !self.story.seen("guerra.rio_cerrado") {
            self.queue_war_scene("guerra.rio_cerrado");
            return;
        }
        if let Some(c) = self.war.as_mut().and_then(|w| w.camp.as_mut()) {
            c.toast = Some((
                "El río espera".into(),
                "Primero el norte: despierta a Varrak (acto 1)".into(),
                0.0,
            ));
        }
    }

    /// The chosen free people woken with her spark (the region becomes hers without a battle).
    pub(super) fn campaign_awaken(&mut self) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        let Some(zone_war::Sel::Province(p)) = c.sel.clone() else {
            return;
        };
        let id = PROVINCES[p].id;
        if s.awaken(id) {
            ui_sfx::play(ui_sfx::UiSfx::Fuse);
            Self::resync_tokens(c, s);
            c.paint_version += 1;
            self.queue_war_scene("guerra.despertar");
            // the second people woken: what the spark costs her
            let n = self.story.get("guerra_despertares") + 1;
            self.story.values.insert("guerra_despertares".into(), n);
            if n >= 2 {
                self.queue_war_scene("guerra.precio_chispa");
            }
        } else {
            ui_sfx::play(ui_sfx::UiSfx::Error);
        }
    }

    /// What the campaign shows of the story: the act's objective, each region's relic.
    pub(super) fn war_story_view(&self) -> zone_war::StoryView {
        let objective = if !self.varrak_awake() {
            "Acto 1 · El norte primero: despierta a Varrak · No pierdas el castillo".to_owned()
        } else if self.story.get("guerra_ganada") > 0 {
            "La capital es tuya · Sigue gobernando".to_owned()
        } else {
            "El río está abierto · Toma la capital caída · No pierdas el castillo".to_owned()
        };
        let relic = relics()
            .iter()
            .map(|r| {
                let found = self.story.get(&format!("reliquia_{}", r.id)) > 0;
                let text = if found {
                    format!("Reliquia encontrada: {}", r.nombre)
                } else if r.on_field() {
                    "Se dice que algo valioso brilla en su campo de batalla.".to_owned()
                } else {
                    "Se dice que quien tome este lugar encontrará algo valioso.".to_owned()
                };
                (r.region.clone(), (text, found))
            })
            .collect();
        zone_war::StoryView {
            objective,
            relic,
            river_closed: !self.varrak_awake(),
            turn_today: self.story.get(TURN_TODAY) > 0,
        }
    }

    // ------------------------------------------------------------------ the war's scenes
    /// Queues a scene of `guerra.md` to play when the war is between moves (once each: a scene
    /// already seen is not queued again; a missing one is skipped).
    pub(super) fn queue_war_scene(&mut self, id: &str) {
        if self.story.seen(id) || guion::library().find(id, None).is_none() {
            return;
        }
        if let Some(w) = self.war.as_mut() {
            if !w.scenes.iter().any(|s| s == id) {
                w.scenes.push_back(id.to_owned());
            }
        }
    }

    /// Plays the next queued scene if nothing is in the way (no conversation, no card open, the
    /// others not moving, no battle being fought).
    pub(super) fn play_war_scenes(&mut self) {
        if self.talk.is_some() || self.stage.is_some() {
            return;
        }
        let Some(w) = self.war.as_ref() else {
            return;
        };
        let idle = match w.page {
            zone_war::WarPage::Campaign => w.camp.as_ref().is_some_and(|c| {
                c.modal.is_none()
                    && !c.busy
                    && c.steps.is_empty()
                    && c.wait <= 0.0
                    && c.marching.is_empty()
            }),
            zone_war::WarPage::Battle => w
                .battle
                .as_ref()
                .is_some_and(|b| b.b.deploying || b.b.paused || b.result.is_some()),
            zone_war::WarPage::Table(_) => true,
        };
        if !idle {
            return;
        }
        let Some(id) = self.war.as_mut().and_then(|w| w.scenes.pop_front()) else {
            return;
        };
        if !self.story.seen(&id) {
            self.start_talk_at(&id);
        }
    }

    /// Everything the war shares with the rest of the game, each step.
    pub(super) fn war_story_tick(&mut self) {
        if self
            .war
            .as_ref()
            .is_some_and(|w| w.page == zone_war::WarPage::Campaign)
        {
            self.campaign_into_story();
        }
        self.treasury_out();
        self.play_war_scenes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_relic_lies_somewhere_real_and_gives_something() {
        let all = relics();
        assert!(
            all.len() >= 10,
            "the relics' file reads ({} relics)",
            all.len()
        );
        let lib = guion::library();
        for r in all {
            assert!(
                PROVINCES.iter().any(|p| p.id == r.region),
                "{}: no region «{}»",
                r.id,
                r.region
            );
            if r.on_field() {
                assert!(
                    wr::baked::map_of_region(&r.region).is_some(),
                    "{}: «{}» has no battlefield to lie on",
                    r.id,
                    r.region
                );
            }
            if let Some(p) = r.recompensa.poder.as_deref() {
                assert!(power_of(p).is_some(), "{}: no power «{p}»", r.id);
            }
            assert!(!r.reward_text().is_empty(), "{} gives nothing", r.id);
            assert!(!r.texto.is_empty() && !r.nombre.is_empty());
        }
        // every scene the war asks for by name is written
        for id in [
            "intro",
            "rio_cerrado",
            "varrak_se_une",
            "primera_victoria",
            "primera_derrota",
            "asedio",
            "despertar",
            "castillo_caido",
            "victoria_final",
            // the second act over the map
            "consejo_traje",
            "el_odio",
            "merin",
            "reliquia_medianoche",
            "precio_chispa",
            "guardia",
            "noche_mesa",
            "nido_llamada",
        ] {
            assert!(
                lib.find(&format!("guerra.{id}"), None).is_some(),
                "guerra.{id} is missing"
            );
        }
        for p in [Power::Grito, Power::Rugido, Power::Bendicion] {
            let id = format!("guerra.poder_{}", power_id(p));
            assert!(lib.find(&id, None).is_some(), "{id} is missing");
        }
    }
}
