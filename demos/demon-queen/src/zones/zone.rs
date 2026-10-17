//! Los lugares del mundo detrás de una misma interfaz.
//!
//! Cada lugar vive en su módulo de `zones/`: el castillo, el bosque, la villa, las termas, el
//! burdel y la mazmorra. Ahí está su paso de juego, el aviso y el uso de F, lo que se oye, cómo se
//! dibuja y el título de la ventana. El mundo guarda el estado de los lugares, y cada uno lo lee y
//! lo cambia a través de la `App` que recibe.
use super::*;

/// Where she is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ZoneId {
    Castle,
    Forest,
    Village,
    Baths,
    Brothel,
    /// El Peñascal, the goblins' village where Varrak sits.
    Penascal,
    /// The fight with Varrak in its plaza, in first person.
    Boss,
    Dungeon,
    /// The war table: its page, the campaign's map, a battle.
    War,
}

/// What every place does in the world.
pub(super) trait Zone: Sync {
    fn id(&self) -> ZoneId;
    /// One step of play here (above ground, after what every place shares: the hour, the camera
    /// settling, the footprints fading).
    fn tick(&self, app: &mut App, dt: f32);
    /// What F would do from where she stands.
    fn prompt(&self, app: &App) -> Option<Prompt>;
    /// F pressed: whether that ends the key's handling (a door, a scene or a conversation took it).
    fn interact(&self, app: &mut App) -> bool;
    /// The music of the place.
    fn soundtrack(&self, app: &App) -> soundtrack::Zone;
    /// The sounds the place itself makes, where they are.
    fn emitters(&self, _app: &App) -> &'static [game_audio::Emitter] {
        &[]
    }
    /// How much of the springs' water is heard.
    fn springs(&self, _app: &App) -> f32 {
        0.0
    }
    /// The ground under her feet, for her steps.
    fn ground(&self, app: &App) -> game_audio::Ground {
        game_audio::Ground::of(self.soundtrack(app))
    }
    /// Draw the frame: the place, the Queen in it, its lights and mood, the overlay.
    fn render(&self, state: &mut RenderState, frame: &Frame);
    /// The standalone preview's title: the place, and a hint of what is in reach.
    fn title(&self, app: &App) -> (&'static str, &'static str);
    /// Read ahead the place behind a door she is walking up to, and let it go if she walks off.
    fn prefetch(&self, _app: &mut App) {}
}

impl App {
    /// Whether reading ahead is on (VELVET_EXP=noprefetch turns it off).
    pub(super) fn prefetch_allowed() -> bool {
        !std::env::var("VELVET_EXP").is_ok_and(|v| v.split(',').any(|f| f == "noprefetch"))
    }
}

impl App {
    /// The place she is in, from the world's state.
    pub(crate) fn zone_id(&self) -> ZoneId {
        if self.dungeon_mode {
            ZoneId::Dungeon
        } else if self.war.is_some() {
            ZoneId::War
        } else if self.boss.is_some() {
            ZoneId::Boss
        } else if self.brothel.is_some() {
            ZoneId::Brothel
        } else if self.baths.is_some() {
            ZoneId::Baths
        } else if self.penascal.is_some() {
            ZoneId::Penascal
        } else if !self.nature_mode {
            ZoneId::Castle
        } else if self.village_entered {
            ZoneId::Village
        } else {
            ZoneId::Forest
        }
    }

    /// The place she is in.
    pub(super) fn zone(&self) -> &'static dyn Zone {
        zone(self.zone_id())
    }

    /// The preview's title out of doors (and at the springs and in the brothel, which open from
    /// the villa): the villa's or the wood's.
    pub(super) fn outdoors_title(&self) -> (&'static str, &'static str) {
        if self.village_entered {
            ("Villa Fantasy [N: terreno]", self.village_title_hint())
        } else {
            ("Mundo natural [N]", self.forest_title_hint())
        }
    }
}

/// Each place by its id.
pub(super) fn zone(id: ZoneId) -> &'static dyn Zone {
    match id {
        ZoneId::Castle => &zone_castle::CastleZone,
        ZoneId::Forest => &zone_forest::ForestZone,
        ZoneId::Village => &zone_village::VillageZone,
        ZoneId::Baths => &zone_baths::BathsZone,
        ZoneId::Brothel => &zone_brothel::BrothelZone,
        ZoneId::Penascal => &zone_penascal::PenascalZone,
        ZoneId::Boss => &zone_boss::BossZone,
        ZoneId::Dungeon => &zone_dungeon::DungeonZone,
        ZoneId::War => &zone_war::WarZone,
    }
}
