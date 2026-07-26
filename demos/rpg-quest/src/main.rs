mod assets;
mod render;

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use assets::{locate_asset_root, AssetStore, ImageAsset};
use render::{rgba, Canvas, FontSystem, Rect, HEIGHT, WIDTH};
use softbuffer::{Context as SoftContext, Surface};
use velvet_math::{Transform2D, Vec2};
use velvet_play::prelude::*;
use velvet_rpg::prelude::*;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

const QUEST_ID: &str = "solaria_liberation";
const MIRA_POS: Vec2 = Vec2::new(480.0, 338.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamePhase {
    Title,
    Playing,
    Victory,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldId {
    Home,
    Forest,
    Camp,
    Cave,
    Castle,
}

impl WorldId {
    fn title(self) -> &'static str {
        match self {
            Self::Home => "MUNDO 1 · LA CASA",
            Self::Forest => "MUNDO 2 · EL BOSQUE",
            Self::Camp => "MUNDO 3 · CAMPAMENTO ENEMIGO",
            Self::Cave => "MUNDO 4 · LA CUEVA",
            Self::Castle => "MUNDO 5 · EL CASTILLO",
        }
    }

    fn objective(self, remaining: usize, boss_active: bool) -> String {
        match self {
            Self::Home => format!("Protege la casa · derrota 3 enemigos ({remaining} restantes)"),
            Self::Forest => {
                format!("Limpia el bosque · derrota 3 enemigos ({remaining} restantes)")
            }
            Self::Camp => {
                format!("Destruye el campamento · derrota 5 enemigos ({remaining} restantes)")
            }
            Self::Cave => format!("Cruza la cueva · derrota 5 enemigos ({remaining} restantes)"),
            Self::Castle if !boss_active => "Habla con el rey dentro del castillo".into(),
            Self::Castle => "Derrota al jefe final".into(),
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            Self::Home => Some(Self::Forest),
            Self::Forest => Some(Self::Camp),
            Self::Camp => Some(Self::Cave),
            Self::Cave => Some(Self::Castle),
            Self::Castle => None,
        }
    }

    fn enemy_count(self) -> usize {
        match self {
            Self::Home | Self::Forest => 3,
            Self::Camp | Self::Cave => 5,
            Self::Castle => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum QuestStage {
    TalkMira,
    DefeatScouts,
    DefeatCaptain,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnemyKind {
    Scout,
    Captain,
}

impl EnemyKind {
    fn max_hp(self) -> f32 {
        match self {
            Self::Scout => 54.0,
            Self::Captain => 150.0,
        }
    }

    fn speed(self) -> f32 {
        match self {
            Self::Scout => 58.0,
            Self::Captain => 44.0,
        }
    }

    fn damage(self) -> f32 {
        match self {
            Self::Scout => 17.0,
            Self::Captain => 28.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Scout => "EXPLORADOR",
            Self::Captain => "CAPITÁN ORCO",
        }
    }
}

#[derive(Debug, Clone)]
struct Enemy {
    entity_id: usize,
    kind: EnemyKind,
    hp: f32,
    max_hp: f32,
    attack_cooldown: f32,
    hurt_timer: f32,
}

impl Enemy {
    fn alive(&self) -> bool {
        self.hp > 0.0
    }
}

#[derive(Debug, Clone, Copy)]
enum DialogueEffect {
    None,
    StartQuest,
}

#[derive(Debug, Clone)]
struct Dialogue {
    speaker: String,
    pages: Vec<String>,
    page: usize,
    effect: DialogueEffect,
}

impl Dialogue {
    fn current(&self) -> &str {
        self.pages.get(self.page).map(String::as_str).unwrap_or("")
    }
}

#[derive(Debug, Clone)]
struct EffectFx {
    position: Vec2,
    timer: f32,
}

struct RpgGameState {
    world: PlayWorld,
    player_id: usize,
    enemies: Vec<Enemy>,
    party: Party,
    item_db: ItemDb,
    journal: QuestJournal,
    world_id: WorldId,
    stage: QuestStage,
    phase: GamePhase,
    keys_pressed: HashSet<KeyCode>,
    dialogue: Option<Dialogue>,
    show_inventory: bool,
    action_log: Vec<String>,
    attack_timer: f32,
    attack_cooldown: f32,
    player_invulnerability: f32,
    anim_time: f32,
    effects: Vec<EffectFx>,
    fullscreen: bool,
}

impl RpgGameState {
    fn new() -> Result<Self> {
        let (world, player_id, enemies) = build_campaign_world(WorldId::Home)?;

        let mut item_db = ItemDb::default();
        item_db.insert(ItemDef::weapon("iron_sword", "Espada de Hierro", 12.0, 25));
        item_db.insert(ItemDef {
            id: ItemId::new("leather_shield"),
            name: "Escudo de Cuero".into(),
            kind: ItemKind::Armor,
            equip_slot: Some(EquipSlot::OffHand),
            max_stack: 1,
            price: 20,
            attack: 0.0,
            defense: 5.0,
            heal: 0.0,
            description: "Un escudo reforzado por Kael".into(),
        });
        item_db.insert(ItemDef::potion("health_potion", "Poción de Vida", 40.0, 10));

        let mut hero = PartyMember::new("hero_valen", "Valen");
        hero.stats.attributes.strength = 9;
        hero.stats.attributes.agility = 7;
        hero.stats.attributes.vitality = 12;
        hero.stats.refill();
        hero.inventory.gold = 25;
        hero.inventory.add("health_potion", 2, 99)?;
        hero.inventory.add("iron_sword", 1, 1)?;
        hero.inventory
            .equip("iron_sword", EquipSlot::MainHand, &item_db)?;
        let mut party = Party::default();
        party.add(hero);

        let mut journal = QuestJournal::default();
        let mut quest = Quest::new(QUEST_ID, "La liberación de Solaria");
        quest.objectives.push(QuestObjective::new(
            "talk_mira",
            "Habla con Mira la Anciana",
            1,
        ));
        quest
            .objectives
            .push(QuestObjective::new("talk_kael", "Recibe equipo de Kael", 1));
        quest.objectives.push(QuestObjective::new(
            "defeat_scouts",
            "Derrota a tres exploradores",
            3,
        ));
        quest.objectives.push(QuestObjective::new(
            "defeat_captain",
            "Derrota al capitán orco",
            1,
        ));
        quest
            .objectives
            .push(QuestObjective::new("return_mira", "Regresa con Mira", 1));
        quest.reward_gold = 120;
        quest.reward_xp = 200;
        journal.start(quest);

        Ok(Self {
            world,
            player_id,
            enemies,
            party,
            item_db,
            journal,
            world_id: WorldId::Home,
            stage: QuestStage::DefeatScouts,
            phase: GamePhase::Title,
            keys_pressed: HashSet::new(),
            dialogue: None,
            show_inventory: false,
            action_log: vec!["Mundo 1: protege la casa y derrota a los tres invasores.".into()],
            attack_timer: 0.0,
            attack_cooldown: 0.0,
            player_invulnerability: 0.0,
            anim_time: 0.0,
            effects: Vec::new(),
            fullscreen: false,
        })
    }

    fn start(&mut self) {
        self.phase = GamePhase::Playing;
        self.add_log("WASD para moverte · E para hablar · Espacio para atacar");
    }

    fn player_pos(&self) -> Vec2 {
        self.world
            .entities
            .get(&self.player_id)
            .map(PlayEntity::position)
            .unwrap_or(Vec2::ZERO)
    }

    fn player_facing(&self) -> Vec2 {
        self.world
            .entities
            .get(&self.player_id)
            .map(|entity| entity.facing.dir.normalize_or_zero())
            .unwrap_or(Vec2::X)
    }

    fn player_is_moving(&self) -> bool {
        self.world
            .entities
            .get(&self.player_id)
            .is_some_and(|entity| entity.velocity.linear.length_squared() > 1.0)
    }

    fn enemy_active(&self, enemy: &Enemy) -> bool {
        match enemy.kind {
            EnemyKind::Scout => true,
            EnemyKind::Captain => self.stage == QuestStage::DefeatCaptain,
        }
    }

    fn scouts_remaining(&self) -> usize {
        self.enemies
            .iter()
            .filter(|enemy| enemy.kind == EnemyKind::Scout && enemy.alive())
            .count()
    }

    fn current_objective(&self) -> String {
        self.world_id.objective(
            self.scouts_remaining(),
            self.stage == QuestStage::DefeatCaptain,
        )
    }

    fn load_world(&mut self, world_id: WorldId) -> Result<()> {
        let (world, player_id, enemies) = build_campaign_world(world_id)?;
        self.world = world;
        self.player_id = player_id;
        self.enemies = enemies;
        self.world_id = world_id;
        self.dialogue = None;
        self.effects.clear();
        self.attack_timer = 0.0;
        self.attack_cooldown = 0.0;
        self.player_invulnerability = 0.8;
        self.stage = if world_id == WorldId::Castle {
            QuestStage::TalkMira
        } else {
            QuestStage::DefeatScouts
        };
        if let Some(leader) = self.party.leader_mut() {
            leader.stats.heal(35.0);
        }
        self.add_log(world_id.title());
        Ok(())
    }

    fn advance_world(&mut self) {
        let Some(next) = self.world_id.next() else {
            return;
        };
        if let Err(error) = self.load_world(next) {
            self.add_log(format!("No se pudo cargar el siguiente mundo: {error}"));
            return;
        }
        match next {
            WorldId::Forest => self.add_log("El sendero conduce a un bosque ocupado."),
            WorldId::Camp => self.add_log("Has encontrado el campamento principal."),
            WorldId::Cave => self.add_log("La salida está al otro lado de la cueva."),
            WorldId::Castle => self.add_log("Busca al rey. Él conoce al jefe enemigo."),
            WorldId::Home => {}
        }
    }

    fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        self.anim_time += dt;
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.player_invulnerability = (self.player_invulnerability - dt).max(0.0);
        for enemy in &mut self.enemies {
            enemy.attack_cooldown = (enemy.attack_cooldown - dt).max(0.0);
            enemy.hurt_timer = (enemy.hurt_timer - dt).max(0.0);
        }
        for effect in &mut self.effects {
            effect.timer -= dt;
        }
        self.effects.retain(|effect| effect.timer > 0.0);

        if self.phase != GamePhase::Playing || self.dialogue.is_some() || self.show_inventory {
            self.world.set_player_input(Vec2::ZERO);
            for enemy in &self.enemies {
                if let Some(entity) = self.world.entities.get_mut(&enemy.entity_id) {
                    entity.velocity = Velocity::ZERO;
                }
            }
            self.world.step(dt);
            return;
        }

        let mut direction = Vec2::ZERO;
        if self.keys_pressed.contains(&KeyCode::KeyW)
            || self.keys_pressed.contains(&KeyCode::ArrowUp)
        {
            direction.y -= 1.0;
        }
        if self.keys_pressed.contains(&KeyCode::KeyS)
            || self.keys_pressed.contains(&KeyCode::ArrowDown)
        {
            direction.y += 1.0;
        }
        if self.keys_pressed.contains(&KeyCode::KeyA)
            || self.keys_pressed.contains(&KeyCode::ArrowLeft)
        {
            direction.x -= 1.0;
        }
        if self.keys_pressed.contains(&KeyCode::KeyD)
            || self.keys_pressed.contains(&KeyCode::ArrowRight)
        {
            direction.x += 1.0;
        }
        self.world.set_player_input(direction.normalize_or_zero());

        let player_pos = self.player_pos();
        let stage = self.stage;
        for enemy in &mut self.enemies {
            let active = match enemy.kind {
                EnemyKind::Scout => true,
                EnemyKind::Captain => stage == QuestStage::DefeatCaptain,
            };
            let Some(entity) = self.world.entities.get_mut(&enemy.entity_id) else {
                continue;
            };
            if !active || !enemy.alive() {
                entity.velocity = Velocity::ZERO;
                continue;
            }
            let offset = player_pos - entity.position();
            let distance = offset.length();
            if distance < 285.0 {
                let dir = offset.normalize_or_zero();
                entity.velocity.linear = dir * enemy.kind.speed();
                entity.facing.dir = dir;
            } else {
                entity.velocity = Velocity::ZERO;
            }
        }

        self.world.step(dt);

        if self.player_invulnerability <= 0.0 {
            let player_pos = self.player_pos();
            let mut incoming: Option<(f32, Vec2, EnemyKind)> = None;
            for enemy in &mut self.enemies {
                if !enemy.alive() || enemy.attack_cooldown > 0.0 {
                    continue;
                }
                let active = match enemy.kind {
                    EnemyKind::Scout => true,
                    EnemyKind::Captain => stage == QuestStage::DefeatCaptain,
                };
                if !active {
                    continue;
                }
                let Some(entity) = self.world.entities.get(&enemy.entity_id) else {
                    continue;
                };
                let offset = player_pos - entity.position();
                if offset.length() < 42.0 {
                    enemy.attack_cooldown = match enemy.kind {
                        EnemyKind::Scout => 1.0,
                        EnemyKind::Captain => 0.72,
                    };
                    incoming = Some((enemy.kind.damage(), offset.normalize_or_zero(), enemy.kind));
                    break;
                }
            }
            if let Some((damage, knock_direction, kind)) = incoming {
                self.damage_player(damage, knock_direction, kind);
            }
        }
    }

    fn damage_player(&mut self, raw_damage: f32, knock_direction: Vec2, kind: EnemyKind) {
        let mut fatal = false;
        let mut hp = 0.0;
        if let Some(leader) = self.party.leader_mut() {
            fatal = leader.stats.take_damage(raw_damage);
            hp = leader.stats.hp;
        }
        if let Some(player) = self.world.entities.get_mut(&self.player_id) {
            player.transform.translation += knock_direction * 16.0;
        }
        self.player_invulnerability = 0.72;
        self.effects.push(EffectFx {
            position: self.player_pos(),
            timer: 0.3,
        });
        self.add_log(format!("{} te golpeó · HP {:.0}", kind.label(), hp));
        if fatal {
            self.phase = GamePhase::GameOver;
            self.world.set_player_input(Vec2::ZERO);
        }
    }

    fn attack(&mut self) {
        if self.phase == GamePhase::Title {
            self.start();
            return;
        }
        if self.phase != GamePhase::Playing || self.show_inventory {
            return;
        }
        if self.dialogue.is_some() {
            self.advance_dialogue();
            return;
        }
        if self.attack_cooldown > 0.0 {
            return;
        }
        self.attack_timer = 0.28;
        self.attack_cooldown = 0.42;

        let player_pos = self.player_pos();
        let facing = self.player_facing();
        let damage = self
            .party
            .leader()
            .map(|leader| leader.stats.attributes.attack() + 12.0)
            .unwrap_or(20.0);

        let targets: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| {
                if !enemy.alive() || !self.enemy_active(enemy) {
                    return None;
                }
                let entity = self.world.entities.get(&enemy.entity_id)?;
                let offset = entity.position() - player_pos;
                let distance = offset.length();
                let dir = offset.normalize_or_zero();
                let dot = facing.x * dir.x + facing.y * dir.y;
                (distance <= 74.0 && dot >= -0.05).then_some(index)
            })
            .collect();

        if targets.is_empty() {
            self.add_log("El ataque no alcanzó a ningún enemigo.");
            return;
        }

        let mut messages = Vec::new();
        let mut scout_kills = 0u32;
        let mut captain_killed = false;
        let mut reward_gold = 0u32;
        let mut reward_xp = 0u32;
        for index in targets {
            let enemy = &mut self.enemies[index];
            enemy.hp = (enemy.hp - damage).max(0.0);
            enemy.hurt_timer = 0.18;
            let entity_pos = self
                .world
                .entities
                .get(&enemy.entity_id)
                .map(PlayEntity::position)
                .unwrap_or(player_pos);
            if let Some(entity) = self.world.entities.get_mut(&enemy.entity_id) {
                entity.transform.translation += facing * 13.0;
            }
            self.effects.push(EffectFx {
                position: entity_pos,
                timer: 0.35,
            });
            if enemy.alive() {
                messages.push(format!(
                    "Golpeaste a {} por {:.0} · HP {:.0}/{:.0}",
                    enemy.kind.label(),
                    damage,
                    enemy.hp,
                    enemy.max_hp
                ));
            } else {
                if let Some(entity) = self.world.entities.get_mut(&enemy.entity_id) {
                    entity.alive = false;
                    entity.velocity = Velocity::ZERO;
                }
                match enemy.kind {
                    EnemyKind::Scout => {
                        scout_kills += 1;
                        reward_gold += 9;
                        reward_xp += 28;
                        messages.push("Explorador derrotado · +9 oro · +28 XP".into());
                    }
                    EnemyKind::Captain => {
                        captain_killed = true;
                        reward_gold += 45;
                        reward_xp += 90;
                        messages.push("¡El capitán cayó! · +45 oro · +90 XP".into());
                    }
                }
            }
        }

        if scout_kills > 0 {
            self.journal
                .progress(QUEST_ID, "defeat_scouts", scout_kills);
        }
        if captain_killed {
            self.journal.progress(QUEST_ID, "defeat_captain", 1);
        }
        let mut level_message = None;
        if let Some(leader) = self.party.leader_mut() {
            leader.inventory.gold += reward_gold;
            let gained = leader.level.add_xp(reward_xp);
            if gained > 0 {
                leader.stats.attributes.strength += gained as i32;
                leader.stats.attributes.vitality += gained as i32;
                leader.stats.refill();
                level_message = Some(format!(
                    "¡Nivel {}! Fuerza y vitalidad aumentaron.",
                    leader.level.level
                ));
            }
        }
        for message in messages {
            self.add_log(message);
        }
        if let Some(message) = level_message {
            self.add_log(message);
        }

        if self.stage == QuestStage::DefeatScouts && self.scouts_remaining() == 0 {
            self.advance_world();
        }
        if self.stage == QuestStage::DefeatCaptain && captain_killed {
            self.stage = QuestStage::Complete;
            self.phase = GamePhase::Victory;
            self.add_log("El jefe cayó. La demo ha terminado.");
        }
    }

    fn interact(&mut self) {
        if self.phase == GamePhase::Title {
            self.start();
            return;
        }
        if self.phase != GamePhase::Playing || self.show_inventory {
            return;
        }
        if self.dialogue.is_some() {
            self.advance_dialogue();
            return;
        }
        if self.world_id == WorldId::Castle
            && self.stage == QuestStage::TalkMira
            && (self.player_pos() - MIRA_POS).length() < 78.0
        {
            self.open_dialogue(
                "Rey Aldren",
                &[
                    "Has cruzado todos los mundos para llegar hasta aquí.",
                    "El jefe enemigo espera frente a la puerta norte. Derrótalo y termina esta guerra.",
                ],
                DialogueEffect::StartQuest,
            );
            return;
        }
        self.add_log("No hay nada con qué interactuar cerca.");
    }

    fn open_dialogue(&mut self, speaker: &str, pages: &[&str], effect: DialogueEffect) {
        self.dialogue = Some(Dialogue {
            speaker: speaker.into(),
            pages: pages.iter().map(|page| (*page).to_owned()).collect(),
            page: 0,
            effect,
        });
        self.world.set_player_input(Vec2::ZERO);
    }

    fn advance_dialogue(&mut self) {
        let Some(dialogue) = &mut self.dialogue else {
            return;
        };
        if dialogue.page + 1 < dialogue.pages.len() {
            dialogue.page += 1;
            return;
        }
        let effect = dialogue.effect;
        self.dialogue = None;
        self.apply_dialogue_effect(effect);
    }

    fn apply_dialogue_effect(&mut self, effect: DialogueEffect) {
        match effect {
            DialogueEffect::StartQuest if self.world_id == WorldId::Castle => {
                self.stage = QuestStage::DefeatCaptain;
                let boss =
                    spawn_enemy(&mut self.world, EnemyKind::Captain, Vec2::new(735.0, 300.0));
                self.enemies.push(boss);
                self.add_log("El rey abrió la puerta. El jefe final ha aparecido.");
            }
            DialogueEffect::None | DialogueEffect::StartQuest => {}
        }
    }

    fn use_potion(&mut self) {
        if self.phase != GamePhase::Playing || self.dialogue.is_some() {
            return;
        }
        let mut message = "No tienes pociones disponibles.".to_owned();
        if let Some(leader) = self.party.leader_mut() {
            if leader.stats.hp >= leader.stats.attributes.max_hp() {
                self.add_log("Ya tienes la vida completa.");
                return;
            }
            let hp_before = leader.stats.hp;
            if leader
                .inventory
                .use_consumable("health_potion", &self.item_db, &mut leader.stats)
                .is_ok()
            {
                let healed = leader.stats.hp - hp_before;
                message = format!("Bebiste una poción · +{healed:.0} HP");
            }
        }
        self.add_log(message);
    }

    fn interaction_hint(&self) -> Option<String> {
        if self.phase != GamePhase::Playing || self.dialogue.is_some() || self.show_inventory {
            return None;
        }
        if self.world_id == WorldId::Castle
            && self.stage == QuestStage::TalkMira
            && (self.player_pos() - MIRA_POS).length() < 86.0
        {
            return Some("E  Hablar con el rey".into());
        }
        None
    }

    fn add_log(&mut self, text: impl Into<String>) {
        self.action_log.push(text.into());
        if self.action_log.len() > 5 {
            self.action_log.remove(0);
        }
    }
}

fn spawn_npc(world: &mut PlayWorld, position: Vec2, action: &str) -> usize {
    world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(position),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(9.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: Some(Interactable::new(action, 60.0)),
        alive: true,
    })
}

fn spawn_enemy(world: &mut PlayWorld, kind: EnemyKind, position: Vec2) -> Enemy {
    let entity_id = world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(position),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(9.0))),
        kinematic: Some(KinematicBody::default()),
        speed: Some(Speed(kind.speed())),
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: None,
        alive: true,
    });
    Enemy {
        entity_id,
        kind,
        hp: kind.max_hp(),
        max_hp: kind.max_hp(),
        attack_cooldown: 0.0,
        hurt_timer: 0.0,
    }
}

fn build_campaign_world(world_id: WorldId) -> Result<(PlayWorld, usize, Vec<Enemy>)> {
    let map = TileMap::from_ascii(&build_map_ascii(world_id), 16.0)?;
    let mut world = PlayWorld::new(map);
    let player_id = world.spawn_player(Vec2::new(92.0, 355.0), 176.0);
    if world_id == WorldId::Castle {
        spawn_npc(&mut world, MIRA_POS, "talk_king");
    }

    let positions = [
        Vec2::new(555.0, 250.0),
        Vec2::new(700.0, 385.0),
        Vec2::new(835.0, 245.0),
        Vec2::new(610.0, 470.0),
        Vec2::new(810.0, 490.0),
    ];
    let mut enemies = Vec::new();
    for position in positions.into_iter().take(world_id.enemy_count()) {
        enemies.push(spawn_enemy(&mut world, EnemyKind::Scout, position));
    }
    Ok((world, player_id, enemies))
}

fn build_map_ascii(world_id: WorldId) -> String {
    let mut grid = vec![vec!['.'; 60]; 40];
    for row in &mut grid {
        row[0] = '#';
        row[59] = '#';
    }
    grid[0].fill('#');
    grid[39].fill('#');

    match world_id {
        WorldId::Home => {
            for row in grid.iter_mut().take(17).skip(6) {
                for cell in row.iter_mut().take(29).skip(15) {
                    *cell = '#';
                }
            }
        }
        WorldId::Forest => {
            for (x, y) in [(14, 8), (22, 18), (35, 10), (44, 23), (29, 30)] {
                for row in grid.iter_mut().skip(y).take(3) {
                    for cell in row.iter_mut().skip(x).take(3) {
                        *cell = '#';
                    }
                }
            }
        }
        WorldId::Camp => {
            for row in grid.iter_mut().take(15).skip(6) {
                for cell in row.iter_mut().take(48).skip(37) {
                    *cell = '#';
                }
            }
        }
        WorldId::Cave => {
            for row in grid.iter_mut().take(33).skip(7) {
                row[14] = '#';
                row[45] = '#';
            }
        }
        WorldId::Castle => {
            for row in grid.iter_mut().take(17).skip(4) {
                for cell in row.iter_mut().take(44).skip(17) {
                    *cell = '#';
                }
            }
        }
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

struct GameAssets {
    ground: Option<ImageAsset>,
    ground_forest: Option<ImageAsset>,
    ground_camp: Option<ImageAsset>,
    ground_cave: Option<ImageAsset>,
    ground_castle: Option<ImageAsset>,
    player_idle: Option<ImageAsset>,
    player_run: Option<ImageAsset>,
    player_attack: Option<ImageAsset>,
    kael: Option<ImageAsset>,
    scout_idle: Option<ImageAsset>,
    scout_run: Option<ImageAsset>,
    captain_idle: Option<ImageAsset>,
    captain_run: Option<ImageAsset>,
    captain_attack: Option<ImageAsset>,
    house_mira: Option<ImageAsset>,
    red_barracks: Option<ImageAsset>,
    red_tower: Option<ImageAsset>,
    cave_gate: Option<ImageAsset>,
    royal_castle: Option<ImageAsset>,
    tree: Option<ImageAsset>,
    bush: Option<ImageAsset>,
    explosion: Option<ImageAsset>,

    ui_frame_main: Option<ImageAsset>,
    ui_frame_alt: Option<ImageAsset>,
    ui_frame_panel: Option<ImageAsset>,
    ui_frame_detail: Option<ImageAsset>,
    ui_frame_profile: Option<ImageAsset>,
    ui_frame_info: Option<ImageAsset>,
    ui_frame_list: Option<ImageAsset>,
    ui_frame_slot: Option<ImageAsset>,
    ui_frame_slot_selected: Option<ImageAsset>,
    ui_frame_slot_locked: Option<ImageAsset>,
    ui_frame_speech: Option<ImageAsset>,
    ui_frame_text: Option<ImageAsset>,
    ui_frame_title: Option<ImageAsset>,
    ui_frame_banner: Option<ImageAsset>,
    ui_button: Option<ImageAsset>,
    ui_button_selected: Option<ImageAsset>,
    ui_bar: Option<ImageAsset>,
    ui_bar_red: Option<ImageAsset>,
    ui_bar_green: Option<ImageAsset>,
    ui_bar_gold: Option<ImageAsset>,
    ui_icon_heart: Option<ImageAsset>,
    ui_icon_coin: Option<ImageAsset>,
    ui_icon_star: Option<ImageAsset>,
    ui_icon_accept: Option<ImageAsset>,
    ui_icon_cancel: Option<ImageAsset>,
    ui_icon_play: Option<ImageAsset>,
    ui_icon_restart: Option<ImageAsset>,
    ui_icon_home: Option<ImageAsset>,
    ui_keycap: Option<ImageAsset>,

    icon_backpack: Option<ImageAsset>,
    icon_potion: Option<ImageAsset>,
    icon_document: Option<ImageAsset>,
    icon_trophy: Option<ImageAsset>,
    icon_skull: Option<ImageAsset>,
    icon_hammer: Option<ImageAsset>,
    icon_gear: Option<ImageAsset>,
    icon_book: Option<ImageAsset>,
}

impl GameAssets {
    fn load() -> Self {
        let mut store = AssetStore::new(locate_asset_root());
        let tiny = "tiny-swords/Tiny Swords (Free Pack)";
        let rune = "ui/runewood";
        Self {
            ground: store.get_cloned(&format!("{tiny}/Terrain/Tileset/Tilemap_color2.png")),
            ground_forest: store.get_cloned(&format!("{tiny}/Terrain/Tileset/Tilemap_color1.png")),
            ground_camp: store.get_cloned(&format!("{tiny}/Terrain/Tileset/Tilemap_color3.png")),
            ground_cave: store.get_cloned(&format!("{tiny}/Terrain/Tileset/Tilemap_color5.png")),
            ground_castle: store.get_cloned(&format!("{tiny}/Terrain/Tileset/Tilemap_color4.png")),
            player_idle: store
                .get_cloned(&format!("{tiny}/Units/Blue Units/Warrior/Warrior_Idle.png")),
            player_run: store
                .get_cloned(&format!("{tiny}/Units/Blue Units/Warrior/Warrior_Run.png")),
            player_attack: store.get_cloned(&format!(
                "{tiny}/Units/Blue Units/Warrior/Warrior_Attack1.png"
            )),
            kael: store.get_cloned(&format!(
                "{tiny}/Units/Yellow Units/Pawn/Pawn_Idle Gold.png"
            )),
            scout_idle: store.get_cloned(&format!("{tiny}/Units/Red Units/Pawn/Pawn_Idle.png")),
            scout_run: store.get_cloned(&format!("{tiny}/Units/Red Units/Pawn/Pawn_Run.png")),
            captain_idle: store
                .get_cloned(&format!("{tiny}/Units/Red Units/Warrior/Warrior_Idle.png")),
            captain_run: store
                .get_cloned(&format!("{tiny}/Units/Red Units/Warrior/Warrior_Run.png")),
            captain_attack: store.get_cloned(&format!(
                "{tiny}/Units/Red Units/Warrior/Warrior_Attack1.png"
            )),
            house_mira: store.get_cloned(&format!("{tiny}/Buildings/Blue Buildings/House1.png")),
            red_barracks: store.get_cloned(&format!("{tiny}/Buildings/Red Buildings/Barracks.png")),
            red_tower: store.get_cloned(&format!("{tiny}/Buildings/Red Buildings/Tower.png")),
            cave_gate: store.get_cloned(&format!("{tiny}/Buildings/Black Buildings/Monastery.png")),
            royal_castle: store.get_cloned(&format!("{tiny}/Buildings/Blue Buildings/Castle.png")),
            tree: store.get_cloned(&format!("{tiny}/Terrain/Resources/Wood/Trees/Tree1.png")),
            bush: store.get_cloned(&format!("{tiny}/Terrain/Decorations/Bushes/Bushe1.png")),
            explosion: store.get_cloned(&format!("{tiny}/Particle FX/Explosion_01.png")),

            ui_frame_main: store.get_cloned(&format!("{rune}/UI_Runewood_Frame01a.png")),
            ui_frame_alt: store.get_cloned(&format!("{rune}/UI_Runewood_Frame02a.png")),
            ui_frame_panel: store.get_cloned(&format!("{rune}/UI_Runewood_Frame03a.png")),
            ui_frame_detail: store.get_cloned(&format!("{rune}/UI_Runewood_Frame04a.png")),
            ui_frame_profile: store.get_cloned(&format!("{rune}/UI_Runewood_FrameSlot03b.png")),
            ui_frame_info: store.get_cloned(&format!("{rune}/UI_Runewood_FrameInfo01b.png")),
            ui_frame_list: store.get_cloned(&format!("{rune}/UI_Runewood_FrameList01c.png")),
            ui_frame_slot: store.get_cloned(&format!("{rune}/UI_Runewood_FrameSlot03b.png")),
            ui_frame_slot_selected: store
                .get_cloned(&format!("{rune}/UI_Runewood_FrameSlot01b.png")),
            ui_frame_slot_locked: store.get_cloned(&format!("{rune}/UI_Runewood_FrameSlot03a.png")),
            ui_frame_speech: store.get_cloned(&format!("{rune}/UI_Runewood_Frame01a.png")),
            ui_frame_text: store.get_cloned(&format!("{rune}/UI_Runewood_Frame05a.png")),
            ui_frame_title: store.get_cloned(&format!("{rune}/UI_Runewood_FrameMarker02a.png")),
            ui_frame_banner: store.get_cloned(&format!("{rune}/UI_Runewood_FrameMarker02b.png")),
            ui_button: store.get_cloned(&format!("{rune}/UI_Runewood_Button01b.png")),
            ui_button_selected: store.get_cloned(&format!("{rune}/UI_Runewood_Button02b.png")),
            ui_bar: store.get_cloned(&format!("{rune}/UI_Runewood_Bar01a.png")),
            ui_bar_red: store.get_cloned(&format!("{rune}/UI_Runewood_BarFiller02a.png")),
            ui_bar_green: store.get_cloned(&format!("{rune}/UI_Runewood_BarFiller02b.png")),
            ui_bar_gold: store.get_cloned(&format!("{rune}/UI_Runewood_BarFiller02c.png")),
            ui_icon_heart: store.get_cloned(&format!("{rune}/UI_Runewood_IconHeart01a.png")),
            ui_icon_coin: store.get_cloned(&format!("{rune}/UI_Runewood_IconCoin01a.png")),
            ui_icon_star: store.get_cloned(&format!("{rune}/UI_Runewood_IconStar01a.png")),
            ui_icon_accept: store.get_cloned(&format!("{rune}/UI_Runewood_IconAccept01a.png")),
            ui_icon_cancel: store.get_cloned(&format!("{rune}/UI_Runewood_IconCancel01a.png")),
            ui_icon_play: store.get_cloned(&format!("{rune}/UI_Runewood_IconPlay01a.png")),
            ui_icon_restart: store.get_cloned(&format!("{rune}/UI_Runewood_IconRestart01a.png")),
            ui_icon_home: store.get_cloned(&format!("{rune}/UI_Runewood_IconHome01a.png")),
            ui_keycap: store.get_cloned(&format!("{rune}/UI_Runewood_Command_KeyboardKey01a.png")),

            icon_backpack: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Backpack.png"),
            icon_potion: store.get_cloned("icons/Icons_Essential/v1.2/Icons/PotionRed.png"),
            icon_document: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Document.png"),
            icon_trophy: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Trophy.png"),
            icon_skull: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Skull.png"),
            icon_hammer: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Hammer.png"),
            icon_gear: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Gear.png"),
            icon_book: store.get_cloned("icons/Icons_Essential/v1.2/Icons/Book.png"),
        }
    }
}

struct App {
    state: RpgGameState,
    assets: GameAssets,
    fonts: FontSystem,
    canvas: Canvas,
    window: Option<Arc<Window>>,
    context: Option<SoftContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last_frame: Instant,
}

impl App {
    fn new() -> Result<Self> {
        Ok(Self {
            state: RpgGameState::new()?,
            assets: GameAssets::load(),
            fonts: FontSystem::load_system()?,
            canvas: Canvas::new(),
            window: None,
            context: None,
            surface: None,
            last_frame: Instant::now(),
        })
    }

    fn reset(&mut self) {
        if let Ok(state) = RpgGameState::new() {
            self.state = state;
            self.state.start();
        }
    }

    fn render(&mut self) {
        self.render_world();
        match self.state.phase {
            GamePhase::Title => self.render_title(),
            GamePhase::Playing => {
                self.render_hud();
                if self.state.show_inventory {
                    self.render_inventory();
                } else if self.state.dialogue.is_some() {
                    self.render_dialogue();
                }
            }
            GamePhase::Victory => self.render_victory(),
            GamePhase::GameOver => self.render_game_over(),
        }
    }

    fn render_world(&mut self) {
        self.canvas.clear(0xff315d32);
        let ground = match self.state.world_id {
            WorldId::Home => self.assets.ground.as_ref(),
            WorldId::Forest => self.assets.ground_forest.as_ref(),
            WorldId::Camp => self.assets.ground_camp.as_ref(),
            WorldId::Cave => self.assets.ground_cave.as_ref(),
            WorldId::Castle => self.assets.ground_castle.as_ref(),
        };
        if let Some(ground) = ground {
            for row in 0..10 {
                for column in 0..15 {
                    self.canvas.image_region(
                        ground,
                        64,
                        64,
                        64,
                        64,
                        Rect::new(column * 64, row * 64, 64, 64),
                        false,
                    );
                }
            }
        }

        self.render_world_decorations();

        for enemy in &self.state.enemies {
            if !enemy.alive() || !self.state.enemy_active(enemy) {
                continue;
            }
            let Some(entity) = self.state.world.entities.get(&enemy.entity_id) else {
                continue;
            };
            let pos = entity.position();
            let moving = entity.velocity.linear.length_squared() > 1.0;
            let flip = entity.facing.dir.x < 0.0;
            let frame = (self.state.anim_time * if moving { 9.0 } else { 5.0 }) as usize;
            let (sprite, size) = match enemy.kind {
                EnemyKind::Scout => (
                    if moving {
                        self.assets
                            .scout_run
                            .as_ref()
                            .or(self.assets.scout_idle.as_ref())
                    } else {
                        self.assets.scout_idle.as_ref()
                    },
                    66,
                ),
                EnemyKind::Captain => (
                    if enemy.attack_cooldown > 0.52 {
                        self.assets
                            .captain_attack
                            .as_ref()
                            .or(self.assets.captain_idle.as_ref())
                    } else if moving {
                        self.assets
                            .captain_run
                            .as_ref()
                            .or(self.assets.captain_idle.as_ref())
                    } else {
                        self.assets.captain_idle.as_ref()
                    },
                    82,
                ),
            };
            self.canvas.alpha_rect(
                Rect::new(
                    pos.x as i32 - size / 3,
                    pos.y as i32 + size / 3,
                    size * 2 / 3,
                    8,
                ),
                rgba(0, 0, 0, 70),
            );
            if let Some(sprite) = sprite {
                self.canvas.sprite_frame(
                    sprite,
                    192,
                    frame,
                    Rect::new(pos.x as i32 - size / 2, pos.y as i32 - size / 2, size, size),
                    flip,
                );
            } else {
                self.canvas
                    .circle(pos.x as i32, pos.y as i32, 15, 0xffb83232);
            }
            let bar_w = if enemy.kind == EnemyKind::Captain {
                74
            } else {
                50
            };
            self.canvas.bar(
                Rect::new(
                    pos.x as i32 - bar_w / 2,
                    pos.y as i32 - size / 2 - 10,
                    bar_w,
                    6,
                ),
                enemy.hp / enemy.max_hp,
                if enemy.hurt_timer > 0.0 {
                    0xffffffff
                } else {
                    0xffe34f4f
                },
            );
            self.canvas.text_line(
                self.fonts.font(true),
                enemy.kind.label(),
                pos.x as i32 - bar_w / 2,
                pos.y as i32 - size / 2 - 16,
                10.0,
                0xffffd2d2,
            );
        }

        if let Some(player) = self.state.world.entities.get(&self.state.player_id) {
            let pos = player.position();
            let flip = player.facing.dir.x < 0.0;
            let moving = self.state.player_is_moving();
            let (sprite, frame) = if self.state.attack_timer > 0.0 {
                let progress = 1.0 - self.state.attack_timer / 0.28;
                (
                    self.assets.player_attack.as_ref(),
                    (progress * 4.0) as usize,
                )
            } else if moving {
                (
                    self.assets
                        .player_run
                        .as_ref()
                        .or(self.assets.player_idle.as_ref()),
                    (self.state.anim_time * 10.0) as usize,
                )
            } else {
                (
                    self.assets.player_idle.as_ref(),
                    (self.state.anim_time * 5.0) as usize,
                )
            };
            self.canvas.alpha_rect(
                Rect::new(pos.x as i32 - 19, pos.y as i32 + 20, 38, 9),
                rgba(0, 0, 0, 75),
            );
            if let Some(sprite) = sprite {
                self.canvas.sprite_frame(
                    sprite,
                    192,
                    frame,
                    Rect::new(pos.x as i32 - 38, pos.y as i32 - 38, 76, 76),
                    flip,
                );
            }
            if self.state.player_invulnerability > 0.0
                && ((self.state.anim_time * 18.0) as i32 % 2 == 0)
            {
                self.canvas.border(
                    Rect::new(pos.x as i32 - 30, pos.y as i32 - 30, 60, 60),
                    2,
                    0xffffd0d0,
                );
            }
        }

        for effect in &self.state.effects {
            if let Some(explosion) = &self.assets.explosion {
                let frame = ((1.0 - effect.timer / 0.35) * 8.0).max(0.0) as usize;
                self.canvas.sprite_frame(
                    explosion,
                    192,
                    frame,
                    Rect::new(
                        effect.position.x as i32 - 42,
                        effect.position.y as i32 - 42,
                        84,
                        84,
                    ),
                    false,
                );
            }
        }

        if let Some(hint) = self.state.interaction_hint() {
            let width = (Canvas::measure_text(self.fonts.font(true), &hint, 14.0) as i32 + 92)
                .clamp(260, 470);
            let rect = Rect::new((WIDTH as i32 - width) / 2, 497, width, 42);
            draw_runewood_panel(
                &mut self.canvas,
                self.assets.ui_frame_text.as_ref(),
                rect,
                8,
                10,
                0xe92a1714,
                0xffb76e3f,
            );
            draw_keycap(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_keycap.as_ref(),
                Rect::new(rect.x + 12, rect.y + 8, 28, 28),
                "E",
            );
            self.canvas.text_line(
                self.fonts.font(true),
                hint.trim_start_matches("E  "),
                rect.x + 53,
                rect.y + 27,
                14.0,
                0xffffe5be,
            );
        }
    }

    fn render_world_decorations(&mut self) {
        match self.state.world_id {
            WorldId::Home => {
                draw_dirt_paths(&mut self.canvas);
                if let Some(house) = &self.assets.house_mira {
                    self.canvas.image_fit(house, Rect::new(312, 52, 330, 275));
                }
                self.draw_nature(
                    &[(35, 65), (760, 60), (50, 420), (835, 430)],
                    &[(180, 380), (690, 395)],
                );
            }
            WorldId::Forest => {
                self.draw_nature(
                    &[
                        (20, 28),
                        (145, 55),
                        (300, 24),
                        (470, 70),
                        (650, 30),
                        (815, 55),
                        (25, 390),
                        (175, 455),
                        (340, 405),
                        (520, 455),
                        (700, 400),
                        (845, 440),
                    ],
                    &[
                        (115, 245),
                        (245, 325),
                        (415, 205),
                        (560, 350),
                        (745, 255),
                        (835, 330),
                    ],
                );
                self.canvas
                    .alpha_rect(Rect::new(0, 0, 960, 640), rgba(24, 58, 35, 38));
            }
            WorldId::Camp => {
                draw_dirt_paths(&mut self.canvas);
                if let Some(barracks) = &self.assets.red_barracks {
                    self.canvas
                        .image_fit(barracks, Rect::new(560, 55, 220, 220));
                }
                if let Some(tower) = &self.assets.red_tower {
                    self.canvas.image_fit(tower, Rect::new(770, 95, 130, 190));
                }
                self.draw_nature(
                    &[(20, 65), (215, 42), (30, 430), (865, 430)],
                    &[(330, 390), (470, 250)],
                );
            }
            WorldId::Cave => {
                self.canvas
                    .alpha_rect(Rect::new(0, 0, 960, 640), rgba(10, 12, 20, 128));
                if let Some(gate) = &self.assets.cave_gate {
                    self.canvas.image_fit(gate, Rect::new(345, 20, 280, 255));
                }
                for (x, y, r) in [
                    (80, 115, 32),
                    (175, 480, 26),
                    (300, 155, 22),
                    (690, 150, 30),
                    (820, 470, 34),
                    (575, 505, 20),
                ] {
                    self.canvas.circle(x, y, r, 0xff252736);
                    self.canvas.circle(x - 5, y - 6, (r - 7).max(4), 0xff3b3e50);
                }
            }
            WorldId::Castle => {
                draw_dirt_paths(&mut self.canvas);
                if let Some(castle) = &self.assets.royal_castle {
                    self.canvas.image_fit(castle, Rect::new(286, 30, 388, 300));
                }
                self.draw_nature(
                    &[(45, 55), (790, 60), (55, 430), (835, 430)],
                    &[(180, 395), (720, 395)],
                );
                Self::draw_npc(
                    &mut self.canvas,
                    &self.fonts,
                    self.state.anim_time,
                    MIRA_POS,
                    self.assets.kael.as_ref(),
                    "REY ALDREN",
                    0xffffdc62,
                );
            }
        }
    }

    fn draw_nature(&mut self, trees: &[(i32, i32)], bushes: &[(i32, i32)]) {
        if let Some(tree) = &self.assets.tree {
            for &(x, y) in trees {
                let frame = ((self.state.anim_time * 4.0) as usize) % (tree.width / 192).max(1);
                self.canvas
                    .sprite_frame(tree, 192, frame, Rect::new(x, y, 96, 128), false);
            }
        }
        if let Some(bush) = &self.assets.bush {
            for &(x, y) in bushes {
                let frame = ((self.state.anim_time * 5.0) as usize) % (bush.width / 128).max(1);
                self.canvas
                    .sprite_frame(bush, 128, frame, Rect::new(x, y, 64, 64), false);
            }
        }
    }

    fn draw_npc(
        canvas: &mut Canvas,
        fonts: &FontSystem,
        anim_time: f32,
        position: Vec2,
        sprite: Option<&ImageAsset>,
        label: &str,
        color: u32,
    ) {
        canvas.alpha_rect(
            Rect::new(position.x as i32 - 18, position.y as i32 + 18, 36, 8),
            rgba(0, 0, 0, 70),
        );
        if let Some(sprite) = sprite {
            let frame = (anim_time * 4.0) as usize;
            canvas.sprite_frame(
                sprite,
                192,
                frame,
                Rect::new(position.x as i32 - 34, position.y as i32 - 34, 68, 68),
                false,
            );
        }
        canvas.text_line(
            fonts.font(true),
            label,
            position.x as i32 - 20,
            position.y as i32 - 37,
            11.0,
            color,
        );
    }

    fn render_hud(&mut self) {
        let profile_panel = Rect::new(12, 10, 314, 96);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_main.as_ref(),
            profile_panel,
            16,
            18,
            0xee211314,
            0xff9a5b39,
        );
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_profile.as_ref(),
            Rect::new(22, 20, 68, 68),
            8,
            10,
            0xff2a1716,
            0xffb56e43,
        );
        if let Some(sprite) = &self.assets.player_idle {
            self.canvas
                .sprite_frame(sprite, 192, 0, Rect::new(27, 25, 58, 58), false);
        }

        if let Some(leader) = self.state.party.leader() {
            self.canvas.text_line(
                self.fonts.pixel(),
                &format!("VALEN  LV {}", leader.level.level),
                102,
                35,
                20.0,
                0xffffd690,
            );

            draw_small_icon(
                &mut self.canvas,
                self.assets.ui_icon_heart.as_ref(),
                Rect::new(101, 44, 22, 18),
            );
            draw_runewood_bar(
                &mut self.canvas,
                self.assets.ui_bar.as_ref(),
                self.assets.ui_bar_green.as_ref(),
                Rect::new(128, 45, 178, 17),
                leader.stats.hp_fraction(),
                0xff4a8c53,
            );
            self.canvas.text_line(
                self.fonts.font(true),
                &format!(
                    "{:.0}/{:.0}",
                    leader.stats.hp,
                    leader.stats.attributes.max_hp()
                ),
                176,
                59,
                11.0,
                0xffffefdc,
            );

            draw_small_icon(
                &mut self.canvas,
                self.assets.ui_icon_star.as_ref(),
                Rect::new(101, 67, 20, 17),
            );
            draw_runewood_bar(
                &mut self.canvas,
                self.assets.ui_bar.as_ref(),
                self.assets.ui_bar_gold.as_ref(),
                Rect::new(128, 68, 112, 14),
                leader.level.xp as f32 / leader.level.xp_to_next.max(1) as f32,
                0xffd69a47,
            );
            self.canvas.text_line(
                self.fonts.font(false),
                &format!("{}/{} XP", leader.level.xp, leader.level.xp_to_next),
                245,
                80,
                10.5,
                0xffe8cda7,
            );

            draw_resource_chip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_list.as_ref(),
                self.assets.ui_icon_coin.as_ref(),
                Rect::new(101, 85, 86, 20),
                &leader.inventory.gold.to_string(),
            );
            draw_resource_chip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_list.as_ref(),
                self.assets.icon_potion.as_ref(),
                Rect::new(192, 85, 106, 20),
                &format!("x{}", leader.inventory.count("health_potion")),
            );
        }

        let quest_panel = Rect::new(684, 10, 264, 72);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_text.as_ref(),
            quest_panel,
            8,
            12,
            0xee251416,
            0xff9e603d,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_title.as_ref(),
            Rect::new(724, 6, 184, 26),
            "MISION ACTIVA",
            12.5,
        );
        let objective = self.state.current_objective();
        self.canvas.text_wrapped(
            self.fonts.font(true),
            &objective,
            700,
            34,
            230,
            12.0,
            16,
            0xffffe9c8,
        );

        if self.state.stage == QuestStage::DefeatCaptain {
            if let Some(captain) = self
                .state
                .enemies
                .iter()
                .find(|enemy| enemy.kind == EnemyKind::Captain && enemy.alive())
            {
                let boss = Rect::new(336, 10, 278, 54);
                draw_runewood_panel(
                    &mut self.canvas,
                    self.assets.ui_frame_info.as_ref(),
                    boss,
                    8,
                    10,
                    0xee251416,
                    0xffa64a3f,
                );
                self.canvas.text_line(
                    self.fonts.pixel(),
                    "CAPITAN ORCO",
                    405,
                    29,
                    16.0,
                    0xffffc4b2,
                );
                draw_runewood_bar(
                    &mut self.canvas,
                    self.assets.ui_bar.as_ref(),
                    self.assets.ui_bar_red.as_ref(),
                    Rect::new(360, 35, 230, 16),
                    captain.hp / captain.max_hp,
                    0xffa73b3b,
                );
            }
        }

        if self.state.dialogue.is_none() && !self.state.show_inventory {
            let log_panel = Rect::new(12, 548, 372, 80);
            self.canvas.alpha_rect(
                Rect::new(
                    log_panel.x + 8,
                    log_panel.y + 8,
                    log_panel.w - 16,
                    log_panel.h - 16,
                ),
                rgba(24, 11, 14, 238),
            );
            draw_runewood_panel(
                &mut self.canvas,
                self.assets.ui_frame_list.as_ref(),
                log_panel,
                8,
                10,
                0xf0180b0e,
                0xffa86a43,
            );
            draw_title_strip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_title.as_ref(),
                Rect::new(28, 540, 130, 26),
                "REGISTRO",
                12.0,
            );
            if let Some(log) = self.state.action_log.last() {
                self.canvas.text_wrapped(
                    self.fonts.font(true),
                    log,
                    29,
                    577,
                    338,
                    14.0,
                    19,
                    0xff12090b,
                );
                self.canvas.text_wrapped(
                    self.fonts.font(true),
                    log,
                    28,
                    576,
                    338,
                    14.0,
                    19,
                    0xfffff0d8,
                );
            }

            let hotbar_panel = Rect::new(398, 548, 550, 80);
            self.canvas.alpha_rect(
                Rect::new(
                    hotbar_panel.x + 8,
                    hotbar_panel.y + 8,
                    hotbar_panel.w - 16,
                    hotbar_panel.h - 16,
                ),
                rgba(24, 11, 14, 238),
            );
            draw_runewood_panel(
                &mut self.canvas,
                self.assets.ui_frame_list.as_ref(),
                hotbar_panel,
                8,
                10,
                0xf0180b0e,
                0xffa86a43,
            );

            let hotbar_x = 468;
            let hotbar_y = 554;
            let slots = [
                ("E", self.assets.ui_icon_accept.as_ref(), "HABLAR"),
                ("SPC", self.assets.icon_hammer.as_ref(), "ATACAR"),
                ("H", self.assets.icon_potion.as_ref(), "POCIÓN"),
                ("I", self.assets.icon_backpack.as_ref(), "MOCHILA"),
            ];
            for (index, (key, icon, label)) in slots.into_iter().enumerate() {
                let x = hotbar_x + index as i32 * 92;
                draw_item_slot(
                    &mut self.canvas,
                    self.assets.ui_frame_slot.as_ref(),
                    icon,
                    Rect::new(x, hotbar_y, 52, 52),
                    false,
                );
                draw_keycap(
                    &mut self.canvas,
                    &self.fonts,
                    self.assets.ui_keycap.as_ref(),
                    Rect::new(x + 34, hotbar_y - 5, 30, 30),
                    key,
                );
                let width = Canvas::measure_text(self.fonts.pixel(), label, 11.5) as i32;
                let label_x = x + (52 - width) / 2;
                self.canvas.text_line(
                    self.fonts.pixel(),
                    label,
                    label_x + 1,
                    622,
                    11.5,
                    0xff12090b,
                );
                self.canvas
                    .text_line(self.fonts.pixel(), label, label_x, 621, 11.5, 0xffffe1ad);
            }
        }
    }

    fn render_dialogue(&mut self) {
        let Some(dialogue) = &self.state.dialogue else {
            return;
        };
        self.canvas
            .alpha_rect(Rect::new(0, 405, 960, 235), rgba(15, 8, 10, 95));

        let portrait_rect = Rect::new(28, 438, 118, 154);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_profile.as_ref(),
            portrait_rect,
            8,
            14,
            0xff241416,
            0xffa96440,
        );
        let portrait = if dialogue.speaker.starts_with("Rey") {
            self.assets.kael.as_ref()
        } else {
            None
        };
        if let Some(portrait) = portrait {
            self.canvas
                .sprite_frame(portrait, 192, 0, Rect::new(36, 448, 102, 102), false);
        }

        let panel = Rect::new(132, 430, 800, 174);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_speech.as_ref(),
            panel,
            8,
            14,
            0xf0261618,
            0xffa96440,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_title.as_ref(),
            Rect::new(166, 420, 310, 34),
            &dialogue.speaker.to_uppercase(),
            16.0,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            dialogue.current(),
            174,
            470,
            704,
            18.0,
            25,
            0xffffead1,
        );

        let page_y = 560;
        for page in 0..dialogue.pages.len() {
            let color = if page == dialogue.page {
                0xffffbd66
            } else {
                0xff6e4135
            };
            self.canvas
                .rect(Rect::new(178 + page as i32 * 14, page_y, 8, 8), color);
        }
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button_selected.as_ref(),
            self.assets.ui_icon_accept.as_ref(),
            Rect::new(710, 548, 188, 40),
            "CONTINUAR",
            14.0,
        );
    }

    fn render_inventory(&mut self) {
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(9, 5, 8, 205),
        );
        let outer = Rect::new(38, 48, 884, 544);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_main.as_ref(),
            outer,
            16,
            22,
            0xff1e1114,
            0xff9c5e3b,
        );

        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_title.as_ref(),
            Rect::new(68, 42, 248, 36),
            "INVENTARIO",
            18.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button_selected.as_ref(),
            self.assets.icon_backpack.as_ref(),
            Rect::new(330, 54, 168, 38),
            "OBJETOS",
            13.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.icon_gear.as_ref(),
            Rect::new(506, 54, 168, 38),
            "EQUIPO",
            13.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.icon_book.as_ref(),
            Rect::new(682, 54, 168, 38),
            "MISION",
            13.0,
        );

        let profile = Rect::new(58, 104, 220, 454);
        let grid = Rect::new(288, 104, 356, 454);
        let detail = Rect::new(654, 104, 248, 454);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_alt.as_ref(),
            profile,
            16,
            18,
            0xff241416,
            0xff915637,
        );
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_panel.as_ref(),
            grid,
            16,
            18,
            0xff241416,
            0xff915637,
        );
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_detail.as_ref(),
            detail,
            16,
            18,
            0xff241416,
            0xff915637,
        );

        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_profile.as_ref(),
            Rect::new(82, 126, 94, 94),
            8,
            12,
            0xff2a1716,
            0xffb56e43,
        );
        if let Some(sprite) = &self.assets.player_idle {
            self.canvas
                .sprite_frame(sprite, 192, 0, Rect::new(88, 132, 82, 82), false);
        }

        if let Some(leader) = self.state.party.leader() {
            self.canvas
                .text_line(self.fonts.pixel(), "VALEN", 184, 154, 19.0, 0xffffd38f);
            self.canvas.text_line(
                self.fonts.font(true),
                &format!("Nivel {}", leader.level.level),
                184,
                178,
                14.0,
                0xffffead4,
            );
            draw_small_icon(
                &mut self.canvas,
                self.assets.ui_icon_heart.as_ref(),
                Rect::new(82, 232, 22, 18),
            );
            draw_runewood_bar(
                &mut self.canvas,
                self.assets.ui_bar.as_ref(),
                self.assets.ui_bar_green.as_ref(),
                Rect::new(110, 232, 142, 18),
                leader.stats.hp_fraction(),
                0xff4a8c53,
            );
            draw_small_icon(
                &mut self.canvas,
                self.assets.ui_icon_star.as_ref(),
                Rect::new(82, 259, 22, 18),
            );
            draw_runewood_bar(
                &mut self.canvas,
                self.assets.ui_bar.as_ref(),
                self.assets.ui_bar_gold.as_ref(),
                Rect::new(110, 259, 142, 16),
                leader.level.xp as f32 / leader.level.xp_to_next.max(1) as f32,
                0xffd69a47,
            );

            self.canvas
                .text_line(self.fonts.pixel(), "ATRIBUTOS", 82, 310, 14.0, 0xffffc87d);
            let attributes = [
                ("FUERZA", leader.stats.attributes.strength),
                ("AGILIDAD", leader.stats.attributes.agility),
                ("VITALIDAD", leader.stats.attributes.vitality),
            ];
            for (index, (label, value)) in attributes.into_iter().enumerate() {
                let y = 338 + index as i32 * 31;
                self.canvas
                    .text_line(self.fonts.font(false), label, 84, y, 13.0, 0xffd6b99c);
                self.canvas.text_line(
                    self.fonts.pixel(),
                    &value.to_string(),
                    226,
                    y,
                    14.0,
                    0xffffe7bd,
                );
            }

            self.canvas
                .text_line(self.fonts.pixel(), "EQUIPO", 82, 446, 14.0, 0xffffc87d);
            draw_item_slot(
                &mut self.canvas,
                self.assets.ui_frame_slot_selected.as_ref(),
                self.assets.icon_hammer.as_ref(),
                Rect::new(82, 466, 62, 62),
                true,
            );
            draw_item_slot(
                &mut self.canvas,
                if self.state.stage >= QuestStage::DefeatScouts {
                    self.assets.ui_frame_slot.as_ref()
                } else {
                    self.assets.ui_frame_slot_locked.as_ref()
                },
                self.assets.icon_gear.as_ref(),
                Rect::new(154, 466, 62, 62),
                false,
            );

            self.canvas
                .text_line(self.fonts.pixel(), "MOCHILA", 310, 137, 15.0, 0xffffc87d);
            let inventory_items: [(Option<&ImageAsset>, String); 12] = [
                (
                    self.assets.icon_potion.as_ref(),
                    format!("{}", leader.inventory.count("health_potion")),
                ),
                (
                    self.assets.ui_icon_coin.as_ref(),
                    leader.inventory.gold.to_string(),
                ),
                (self.assets.icon_document.as_ref(), "1".into()),
                (self.assets.icon_backpack.as_ref(), "".into()),
                (self.assets.icon_hammer.as_ref(), "1".into()),
                (self.assets.icon_gear.as_ref(), "1".into()),
                (None, "".into()),
                (None, "".into()),
                (None, "".into()),
                (None, "".into()),
                (None, "".into()),
                (None, "".into()),
            ];
            for (index, (icon, count)) in inventory_items.iter().enumerate() {
                let column = index % 4;
                let row = index / 4;
                let rect = Rect::new(310 + column as i32 * 78, 158 + row as i32 * 86, 66, 66);
                draw_item_slot(
                    &mut self.canvas,
                    if index == 0 {
                        self.assets.ui_frame_slot_selected.as_ref()
                    } else {
                        self.assets.ui_frame_slot.as_ref()
                    },
                    *icon,
                    rect,
                    index == 0,
                );
                if !count.is_empty() {
                    draw_count_badge(
                        &mut self.canvas,
                        &self.fonts,
                        self.assets.ui_frame_list.as_ref(),
                        Rect::new(rect.x + 38, rect.y + 44, 30, 20),
                        count,
                    );
                }
            }

            self.canvas.text_line(
                self.fonts.pixel(),
                "POCION DE VIDA",
                680,
                142,
                15.0,
                0xffffc87d,
            );
            draw_item_slot(
                &mut self.canvas,
                self.assets.ui_frame_slot_selected.as_ref(),
                self.assets.icon_potion.as_ref(),
                Rect::new(734, 166, 88, 88),
                true,
            );
            self.canvas.text_wrapped(
                self.fonts.font(false),
                "Restaura 40 puntos de vida. No se consume cuando Valen ya tiene la vida completa.",
                680,
                280,
                196,
                15.0,
                21,
                0xffffead4,
            );
            draw_resource_chip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_list.as_ref(),
                self.assets.icon_potion.as_ref(),
                Rect::new(680, 370, 196, 28),
                &format!("EN MOCHILA: {}", leader.inventory.count("health_potion")),
            );
            draw_button(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_button_selected.as_ref(),
                self.assets.ui_icon_accept.as_ref(),
                Rect::new(680, 420, 196, 46),
                "H  USAR",
                14.0,
            );
            draw_button(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_button.as_ref(),
                self.assets.ui_icon_cancel.as_ref(),
                Rect::new(680, 480, 196, 42),
                "I / ESC  CERRAR",
                12.5,
            );
        }
    }

    fn render_title(&mut self) {
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(12, 6, 10, 178),
        );
        self.canvas.gradient_vertical(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            0x221f0d14,
            0xaa0b070a,
        );

        let title_panel = Rect::new(54, 82, 520, 470);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_main.as_ref(),
            title_panel,
            16,
            24,
            0xe91f1115,
            0xffa7633d,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_banner.as_ref(),
            Rect::new(126, 68, 370, 48),
            "RPG QUEST",
            27.0,
        );
        self.canvas.text_line(
            self.fonts.pixel(),
            "LOS CINCO MUNDOS",
            124,
            151,
            16.0,
            0xffffc87e,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "Una campaña corta a través de una casa, un bosque, un campamento, una cueva y un castillo. Supera cada mundo y derrota al jefe final.",
            108,
            194,
            400,
            17.0,
            24,
            0xffffead4,
        );

        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_profile.as_ref(),
            Rect::new(112, 304, 158, 170),
            8,
            16,
            0xff241416,
            0xffb56e43,
        );
        if let Some(sprite) = &self.assets.player_idle {
            self.canvas
                .sprite_frame(sprite, 192, 0, Rect::new(120, 313, 142, 142), false);
        }
        self.canvas
            .text_line(self.fonts.pixel(), "VALEN", 166, 497, 16.0, 0xffffd690);

        let menu_x = 306;
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button_selected.as_ref(),
            self.assets.ui_icon_play.as_ref(),
            Rect::new(menu_x, 320, 226, 54),
            "NUEVA PARTIDA",
            16.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.icon_gear.as_ref(),
            Rect::new(menu_x, 388, 226, 48),
            "CONTROLES",
            14.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.ui_icon_home.as_ref(),
            Rect::new(menu_x, 450, 226, 48),
            "ESC  SALIR",
            14.0,
        );

        let controls = Rect::new(602, 154, 312, 310);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_text.as_ref(),
            controls,
            8,
            14,
            0xe8241518,
            0xff945a3a,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_title.as_ref(),
            Rect::new(650, 142, 216, 34),
            "CONTROLES",
            16.0,
        );
        let control_rows = [
            ("W", "MOVERSE"),
            ("E", "HABLAR / ABRIR"),
            ("SPC", "ATACAR"),
            ("H", "USAR POCION"),
            ("I", "INVENTARIO"),
        ];
        for (index, (key, label)) in control_rows.into_iter().enumerate() {
            let y = 194 + index as i32 * 48;
            draw_keycap(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_keycap.as_ref(),
                Rect::new(634, y - 24, 38, 38),
                key,
            );
            self.canvas
                .text_line(self.fonts.pixel(), label, 692, y, 13.0, 0xffffd7a8);
        }
        self.canvas.text_line(
            self.fonts.font(false),
            "Enter o Espacio para comenzar",
            640,
            505,
            14.0,
            0xffd4b79a,
        );
        self.canvas.text_line(
            self.fonts.font(false),
            "Velvet Engine · velvet-play · velvet-rpg",
            620,
            532,
            12.0,
            0xff967866,
        );
    }

    fn render_victory(&mut self) {
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(7, 15, 10, 205),
        );
        let panel = Rect::new(238, 108, 484, 424);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_main.as_ref(),
            panel,
            16,
            22,
            0xf01d1215,
            0xff8f7240,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_banner.as_ref(),
            Rect::new(300, 88, 360, 56),
            "JEFE DERROTADO",
            22.0,
        );
        draw_item_slot(
            &mut self.canvas,
            self.assets.ui_frame_slot_selected.as_ref(),
            self.assets.icon_trophy.as_ref(),
            Rect::new(422, 165, 116, 116),
            true,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "Has completado los cinco mundos. El rey está a salvo y el jefe enemigo ha sido derrotado.",
            304,
            304,
            352,
            17.0,
            23,
            0xffffead4,
        );
        if let Some(leader) = self.state.party.leader() {
            draw_resource_chip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_list.as_ref(),
                self.assets.ui_icon_star.as_ref(),
                Rect::new(304, 394, 164, 32),
                &format!("NIVEL {}", leader.level.level),
            );
            draw_resource_chip(
                &mut self.canvas,
                &self.fonts,
                self.assets.ui_frame_list.as_ref(),
                self.assets.ui_icon_coin.as_ref(),
                Rect::new(486, 394, 164, 32),
                &format!("{} ORO", leader.inventory.gold),
            );
        }
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button_selected.as_ref(),
            self.assets.ui_icon_restart.as_ref(),
            Rect::new(304, 452, 164, 48),
            "R  REINICIAR",
            13.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.ui_icon_home.as_ref(),
            Rect::new(486, 452, 164, 48),
            "ESC  MENU",
            13.0,
        );
    }

    fn render_game_over(&mut self) {
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(27, 5, 9, 215),
        );
        let panel = Rect::new(250, 122, 460, 396);
        draw_runewood_panel(
            &mut self.canvas,
            self.assets.ui_frame_main.as_ref(),
            panel,
            16,
            22,
            0xf01d1215,
            0xff9a473e,
        );
        draw_title_strip(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_frame_banner.as_ref(),
            Rect::new(316, 102, 328, 54),
            "VALEN HA CAIDO",
            22.0,
        );
        draw_item_slot(
            &mut self.canvas,
            self.assets.ui_frame_slot_selected.as_ref(),
            self.assets.icon_skull.as_ref(),
            Rect::new(426, 180, 108, 108),
            true,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "Los caminos de Solaria siguen ocupados. Usa las pociones, controla la distancia y vuelve a intentarlo.",
            318,
            312,
            324,
            17.0,
            23,
            0xffffd9d2,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button_selected.as_ref(),
            self.assets.ui_icon_restart.as_ref(),
            Rect::new(298, 438, 168, 48),
            "R  REINTENTAR",
            13.0,
        );
        draw_button(
            &mut self.canvas,
            &self.fonts,
            self.assets.ui_button.as_ref(),
            self.assets.ui_icon_home.as_ref(),
            Rect::new(488, 438, 168, 48),
            "ESC  MENU",
            13.0,
        );
    }

    fn present(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let Some(surface) = &mut self.surface else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let Some(width) = NonZeroU32::new(size.width) else {
            return;
        };
        let Some(height) = NonZeroU32::new(size.height) else {
            return;
        };
        if surface.resize(width, height).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        scale_letterboxed(
            &self.canvas.pixels,
            &mut buffer,
            size.width as usize,
            size.height as usize,
        );
        let _ = buffer.present();
    }

    fn handle_key(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        match key {
            KeyCode::F11 => {
                self.state.fullscreen = !self.state.fullscreen;
                if let Some(window) = &self.window {
                    window.set_fullscreen(if self.state.fullscreen {
                        Some(Fullscreen::Borderless(None))
                    } else {
                        None
                    });
                }
            }
            KeyCode::Escape => {
                if self.state.show_inventory {
                    self.state.show_inventory = false;
                } else if self.state.dialogue.is_some() {
                    self.state.dialogue = None;
                } else if self.state.phase == GamePhase::Title {
                    event_loop.exit();
                } else {
                    self.state.phase = GamePhase::Title;
                }
            }
            KeyCode::Enter => match self.state.phase {
                GamePhase::Title => self.state.start(),
                GamePhase::Victory | GamePhase::GameOver => self.reset(),
                GamePhase::Playing => self.state.interact(),
            },
            KeyCode::Space => self.state.attack(),
            KeyCode::KeyE => self.state.interact(),
            KeyCode::KeyH => self.state.use_potion(),
            KeyCode::KeyI => {
                if self.state.phase == GamePhase::Playing && self.state.dialogue.is_none() {
                    self.state.show_inventory = !self.state.show_inventory;
                }
            }
            KeyCode::KeyR
                if matches!(self.state.phase, GamePhase::Victory | GamePhase::GameOver) =>
            {
                self.reset();
            }
            _ => {}
        }
    }
}

fn draw_runewood_panel(
    canvas: &mut Canvas,
    frame: Option<&ImageAsset>,
    rect: Rect,
    source_border: usize,
    destination_border: i32,
    fallback_fill: u32,
    fallback_border: u32,
) {
    if let Some(frame) = frame {
        canvas.nine_slice(frame, source_border, rect, destination_border);
    } else {
        canvas.alpha_rect(rect, fallback_fill);
        canvas.border(rect, 2, fallback_border);
    }
}

fn draw_title_strip(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    frame: Option<&ImageAsset>,
    rect: Rect,
    label: &str,
    size: f32,
) {
    if let Some(frame) = frame {
        canvas.horizontal_slice(frame, 8, rect, 12);
    } else {
        canvas.alpha_rect(rect, 0xffb76e43);
        canvas.border(rect, 2, 0xff4b2927);
    }
    let width = Canvas::measure_text(fonts.pixel(), label, size).round() as i32;
    canvas.text_line(
        fonts.pixel(),
        label,
        rect.x + (rect.w - width) / 2,
        rect.y + (rect.h + size.round() as i32) / 2 - 1,
        size,
        0xffffd9a5,
    );
}

fn draw_button(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    background: Option<&ImageAsset>,
    icon: Option<&ImageAsset>,
    rect: Rect,
    label: &str,
    size: f32,
) {
    if let Some(background) = background {
        canvas.nine_slice(background, 8, rect, 10);
    } else {
        canvas.alpha_rect(rect, 0xffb86f43);
        canvas.border(rect, 2, 0xff4b2927);
    }
    if let Some(icon) = icon {
        canvas.image_fit(
            icon,
            Rect::new(rect.x + 10, rect.y + 8, rect.h - 16, rect.h - 16),
        );
    }
    let width = Canvas::measure_text(fonts.pixel(), label, size).round() as i32;
    let icon_offset = if icon.is_some() { rect.h / 3 } else { 0 };
    canvas.text_line(
        fonts.pixel(),
        label,
        rect.x + (rect.w - width) / 2 + icon_offset,
        rect.y + (rect.h + size.round() as i32) / 2 - 1,
        size,
        0xffffddb0,
    );
    canvas.border(
        Rect::new(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4),
        1,
        0xff9f603c,
    );
}

fn draw_runewood_bar(
    canvas: &mut Canvas,
    shell: Option<&ImageAsset>,
    filler: Option<&ImageAsset>,
    rect: Rect,
    value: f32,
    fallback_color: u32,
) {
    if let Some(shell) = shell {
        canvas.horizontal_slice(shell, 5, rect, 7);
    } else {
        canvas.alpha_rect(rect, 0xff2a1717);
        canvas.border(rect, 1, 0xff8b5238);
    }
    let fill_width = ((rect.w - 12) as f32 * value.clamp(0.0, 1.0)).round() as i32;
    if fill_width <= 0 {
        return;
    }
    let inner = Rect::new(rect.x + 6, rect.y + 4, fill_width, (rect.h - 8).max(3));
    if let Some(filler) = filler {
        canvas.horizontal_slice(filler, 3, inner, 4);
    } else {
        canvas.rect(inner, fallback_color);
    }
}

fn draw_item_slot(
    canvas: &mut Canvas,
    frame: Option<&ImageAsset>,
    icon: Option<&ImageAsset>,
    rect: Rect,
    selected: bool,
) {
    if let Some(frame) = frame {
        canvas.nine_slice(frame, 5, rect, 8);
    } else {
        canvas.alpha_rect(rect, if selected { 0xff9c5c3a } else { 0xff3a2220 });
        canvas.border(rect, 2, 0xffb66b42);
    }
    if let Some(icon) = icon {
        canvas.image_fit(
            icon,
            Rect::new(rect.x + 9, rect.y + 9, rect.w - 18, rect.h - 18),
        );
    }
    if selected {
        canvas.border(
            Rect::new(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4),
            2,
            0xffffb95f,
        );
    }
}

fn draw_small_icon(canvas: &mut Canvas, icon: Option<&ImageAsset>, rect: Rect) {
    if let Some(icon) = icon {
        canvas.image_fit(icon, rect);
    }
}

fn draw_keycap(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    keycap: Option<&ImageAsset>,
    rect: Rect,
    label: &str,
) {
    if let Some(keycap) = keycap {
        canvas.nine_slice(keycap, 5, rect, 6);
    } else {
        canvas.alpha_rect(rect, 0xff9e5f3e);
        canvas.border(rect, 2, 0xff3a2220);
    }
    let size = if label.len() > 1 { 9.5 } else { 13.0 };
    let width = Canvas::measure_text(fonts.pixel(), label, size).round() as i32;
    canvas.text_line(
        fonts.pixel(),
        label,
        rect.x + (rect.w - width) / 2,
        rect.y + (rect.h + size.round() as i32) / 2 - 1,
        size,
        0xffffdfaa,
    );
}

fn draw_resource_chip(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    frame: Option<&ImageAsset>,
    icon: Option<&ImageAsset>,
    rect: Rect,
    value: &str,
) {
    if let Some(frame) = frame {
        canvas.horizontal_slice(frame, 8, rect, 10);
    } else {
        canvas.alpha_rect(rect, 0xff3a2220);
        canvas.border(rect, 1, 0xff8b5238);
    }
    if let Some(icon) = icon {
        canvas.image_fit(
            icon,
            Rect::new(rect.x + 5, rect.y + 3, rect.h - 6, rect.h - 6),
        );
    }
    canvas.text_line(
        fonts.pixel(),
        value,
        rect.x + rect.h + 3,
        rect.y + (rect.h + 11) / 2,
        11.0,
        0xffffd9a8,
    );
}

fn draw_count_badge(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    frame: Option<&ImageAsset>,
    rect: Rect,
    value: &str,
) {
    if let Some(frame) = frame {
        canvas.horizontal_slice(frame, 8, rect, 8);
    } else {
        canvas.alpha_rect(rect, 0xff4a2924);
    }
    let width = Canvas::measure_text(fonts.pixel(), value, 10.0).round() as i32;
    canvas.text_line(
        fonts.pixel(),
        value,
        rect.x + (rect.w - width) / 2,
        rect.y + 14,
        10.0,
        0xffffdcaa,
    );
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("RPG Quest: La liberación de Solaria — Velvet Engine")
            .with_inner_size(PhysicalSize::new(1100, 735))
            .with_min_inner_size(PhysicalSize::new(900, 600))
            .with_position(PhysicalPosition::new(80, 50));
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                match SoftContext::new(window.clone()).and_then(|context| {
                    Surface::new(&context, window.clone()).map(|surface| (context, surface))
                }) {
                    Ok((context, surface)) => {
                        self.window = Some(window);
                        self.context = Some(context);
                        self.surface = Some(surface);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    Err(error) => {
                        eprintln!("No se pudo crear la superficie: {error}");
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                eprintln!("No se pudo crear la ventana: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.state.keys_pressed.insert(key);
                            if !event.repeat {
                                self.handle_key(key, event_loop);
                            }
                        }
                        ElementState::Released => {
                            self.state.keys_pressed.remove(&key);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.state.update(dt);
                self.render();
                self.present();
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn draw_dirt_paths(canvas: &mut Canvas) {
    let outer = 0xff805b36;
    let earth = 0xffad7d48;
    let light = 0xffc49358;
    let worn = 0xff90663d;

    let segments = [
        (Rect::new(55, 318, 850, 76), Rect::new(61, 326, 838, 60)),
        (Rect::new(184, 230, 64, 122), Rect::new(192, 230, 48, 122)),
        (Rect::new(454, 218, 64, 134), Rect::new(462, 218, 48, 134)),
        (Rect::new(750, 220, 72, 132), Rect::new(759, 220, 54, 132)),
    ];
    for (edge, center) in segments {
        canvas.alpha_rect(edge, rgba(84, 58, 34, 210));
        canvas.rect(center, earth);
        canvas.border(center, 2, outer);
    }

    for (x, y, radius, color) in [
        (82, 345, 4, light),
        (130, 372, 3, worn),
        (174, 338, 3, light),
        (224, 286, 3, worn),
        (278, 362, 4, light),
        (339, 336, 3, worn),
        (401, 375, 3, light),
        (486, 279, 4, worn),
        (540, 345, 3, light),
        (603, 371, 4, worn),
        (675, 338, 3, light),
        (785, 278, 4, worn),
        (839, 366, 3, light),
        (886, 343, 4, worn),
    ] {
        canvas.circle(x, y, radius, color);
    }
}

fn viewport_transform(destination_width: usize, destination_height: usize) -> (f32, f32, f32) {
    let scale = (destination_width as f32 / WIDTH as f32)
        .min(destination_height as f32 / HEIGHT as f32)
        .max(0.001);
    let viewport_width = WIDTH as f32 * scale;
    let viewport_height = HEIGHT as f32 * scale;
    (
        scale,
        (destination_width as f32 - viewport_width) * 0.5,
        (destination_height as f32 - viewport_height) * 0.5,
    )
}

fn scale_letterboxed(
    source: &[u32],
    destination: &mut [u32],
    destination_width: usize,
    destination_height: usize,
) {
    destination.fill(0xff080a0d);
    let (scale, offset_x, offset_y) = viewport_transform(destination_width, destination_height);
    let viewport_width = (WIDTH as f32 * scale).round() as usize;
    let viewport_height = (HEIGHT as f32 * scale).round() as usize;
    let start_x = offset_x.max(0.0).round() as usize;
    let start_y = offset_y.max(0.0).round() as usize;
    for dy in 0..viewport_height.min(destination_height.saturating_sub(start_y)) {
        let source_y = (dy as f32 / scale) as usize;
        for dx in 0..viewport_width.min(destination_width.saturating_sub(start_x)) {
            let source_x = (dx as f32 / scale) as usize;
            destination[(start_y + dy) * destination_width + start_x + dx] =
                source[source_y.min(HEIGHT - 1) * WIDTH + source_x.min(WIDTH - 1)];
        }
    }
}

fn run_headless() -> Result<()> {
    let mut state = RpgGameState::new()?;
    state.start();
    anyhow::ensure!(state.world_id == WorldId::Home);
    anyhow::ensure!(state.scouts_remaining() == 3);

    for expected in [
        WorldId::Forest,
        WorldId::Camp,
        WorldId::Cave,
        WorldId::Castle,
    ] {
        state.advance_world();
        anyhow::ensure!(state.world_id == expected);
        anyhow::ensure!(state.scouts_remaining() == expected.enemy_count());
    }

    anyhow::ensure!(state.stage == QuestStage::TalkMira);
    state.apply_dialogue_effect(DialogueEffect::StartQuest);
    anyhow::ensure!(state.stage == QuestStage::DefeatCaptain);
    anyhow::ensure!(state
        .enemies
        .iter()
        .any(|enemy| enemy.kind == EnemyKind::Captain));

    println!("RPG QUEST HEADLESS OK");
    println!("worlds=5 home=3 forest=3 camp=5 cave=5 castle=boss");
    Ok(())
}

fn run_capture(name: &str, output: &Path) -> Result<()> {
    let mut app = App::new()?;
    app.state.start();
    match name {
        "title" => app.state.phase = GamePhase::Title,
        "gameplay" | "world1" => {}
        "world2" => app.state.load_world(WorldId::Forest)?,
        "world3" => app.state.load_world(WorldId::Camp)?,
        "world4" => app.state.load_world(WorldId::Cave)?,
        "world5" => app.state.load_world(WorldId::Castle)?,
        "dialogue" => {
            app.state.load_world(WorldId::Castle)?;
            app.state.open_dialogue(
                "Rey Aldren",
                &["El jefe enemigo espera frente a la puerta norte."],
                DialogueEffect::None,
            );
        }
        "inventory" => app.state.show_inventory = true,
        "boss" => {
            app.state.load_world(WorldId::Castle)?;
            app.state.apply_dialogue_effect(DialogueEffect::StartQuest);
            if let Some(player) = app.state.world.entities.get_mut(&app.state.player_id) {
                player.transform.translation = Vec2::new(650.0, 360.0);
            }
        }
        "victory" => {
            app.state.stage = QuestStage::Complete;
            app.state.phase = GamePhase::Victory;
        }
        "gameover" => app.state.phase = GamePhase::GameOver,
        other => anyhow::bail!("captura desconocida: {other}"),
    }
    app.render();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("no se pudo crear {}", parent.display()))?;
    }
    let mut bytes = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for pixel in &app.canvas.pixels {
        bytes.push(((pixel >> 16) & 0xff) as u8);
        bytes.push(((pixel >> 8) & 0xff) as u8);
        bytes.push((pixel & 0xff) as u8);
        bytes.push(255);
    }
    image::save_buffer(
        output,
        &bytes,
        WIDTH as u32,
        HEIGHT as u32,
        image::ColorType::Rgba8,
    )?;
    println!("capture={name} -> {}", output.display());
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.iter().any(|argument| argument == "--headless") {
        return run_headless();
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--capture-screen")
    {
        let name = arguments.get(index + 1).context("falta la pantalla")?;
        let output = arguments
            .get(index + 2)
            .context("falta la ruta de salida")?;
        return run_capture(name, Path::new(output));
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_has_five_worlds_with_expected_enemies() {
        let mut state = RpgGameState::new().unwrap();
        assert_eq!(state.world_id, WorldId::Home);
        assert_eq!(state.scouts_remaining(), 3);

        for (world, count) in [
            (WorldId::Forest, 3),
            (WorldId::Camp, 5),
            (WorldId::Cave, 5),
            (WorldId::Castle, 0),
        ] {
            state.load_world(world).unwrap();
            assert_eq!(state.world_id, world);
            assert_eq!(state.scouts_remaining(), count);
        }
    }

    #[test]
    fn king_spawns_final_boss() {
        let mut state = RpgGameState::new().unwrap();
        state.load_world(WorldId::Castle).unwrap();
        assert_eq!(state.stage, QuestStage::TalkMira);
        state.apply_dialogue_effect(DialogueEffect::StartQuest);
        assert_eq!(state.stage, QuestStage::DefeatCaptain);
        assert!(state
            .enemies
            .iter()
            .any(|enemy| enemy.kind == EnemyKind::Captain && enemy.alive()));
    }
}
