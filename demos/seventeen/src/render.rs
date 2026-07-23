use velvet_story::{draw_text_line, draw_text_wrapped, pack_rgb};

use crate::assets::{LocalArt, SpriteSheet};
use crate::model::{EnemyView, EventView, FrameView, PickupView, RectView, Vec2};
use crate::save::Settings;

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 540;
pub const PIXEL_WIDTH: u32 = 480;
pub const PIXEL_HEIGHT: u32 = 270;
const SCALE: f32 = 1.5;
const PIXEL_SCALE: i32 = 2;

const BLACK: (u8, u8, u8) = (5, 5, 8);
const INK: (u8, u8, u8) = (12, 12, 18);
const PANEL: (u8, u8, u8) = (20, 18, 28);
const WHITE: (u8, u8, u8) = (238, 235, 228);
const MUTED: (u8, u8, u8) = (139, 134, 148);
const RED: (u8, u8, u8) = (224, 42, 61);
const VIOLET: (u8, u8, u8) = (124, 69, 201);
const CYAN: (u8, u8, u8) = (81, 218, 210);
const GOLD: (u8, u8, u8) = (239, 186, 73);
const OUTLINE: (u8, u8, u8) = (6, 7, 11);
const STEEL: (u8, u8, u8) = (65, 73, 88);
const STEEL_LIGHT: (u8, u8, u8) = (112, 119, 133);
const BLOOD: (u8, u8, u8) = (111, 15, 32);
const DEEP_VIOLET: (u8, u8, u8) = (45, 27, 66);

#[derive(Clone)]
struct Particle {
    pos: Vec2,
    velocity: Vec2,
    life: f32,
    max_life: f32,
    color: (u8, u8, u8),
    size: f32,
}

pub struct Renderer {
    pub pixels: Vec<u32>,
    art: LocalArt,
    particles: Vec<Particle>,
    time: f32,
    shake: f32,
    flash: f32,
    player_shoot: f32,
    player_dash: f32,
    player_respawn: f32,
    rng: u32,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; (PIXEL_WIDTH * PIXEL_HEIGHT) as usize],
            art: LocalArt::discover(),
            particles: Vec::new(),
            time: 0.0,
            shake: 0.0,
            flash: 0.0,
            player_shoot: 0.0,
            player_dash: 0.0,
            player_respawn: 0.0,
            rng: 17,
        }
    }

    pub fn asset_status(&self) -> String {
        self.art.startup_message()
    }

    pub fn update(&mut self, dt: f32, events: &[EventView], settings: &Settings) {
        self.time += dt;
        self.shake = (self.shake - dt * 9.0).max(0.0);
        self.flash = (self.flash - dt * 4.0).max(0.0);
        self.player_shoot = (self.player_shoot - dt).max(0.0);
        self.player_dash = (self.player_dash - dt).max(0.0);
        self.player_respawn = (self.player_respawn - dt).max(0.0);
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.pos.x += particle.velocity.x * dt;
            particle.pos.y += particle.velocity.y * dt;
            particle.velocity.x *= 0.96;
            particle.velocity.y *= 0.96;
        }
        self.particles.retain(|particle| particle.life > 0.0);
        for event in events {
            self.consume_event(event, settings);
        }
    }

    pub fn paint_splash(&mut self, progress: f32) {
        self.fill(BLACK);
        let glow = (progress * std::f32::consts::PI).sin().max(0.0);
        self.rect(244, 176, 472, 180, OUTLINE);
        self.rect(248, 180, 464, 172, (13, 12, 19));
        for x in (264..704).step_by(22) {
            let height = 12 + ((x / 22) % 5) * 7;
            self.rect(x, 326 - height, 6, height, mix(DEEP_VIOLET, VIOLET, glow));
        }
        let mark = [
            "oo.........oo",
            "omo.......omo",
            ".omo.....omo.",
            ".omo.....omo.",
            "..omo...omo..",
            "..omo...omo..",
            "...omo.omo...",
            "...omo.omo...",
            "....omomo....",
            "....omomo....",
            ".....omo.....",
            ".....omo.....",
            "......o......",
        ];
        self.blit_sprite_scaled(181, 130, &mark, VIOLET, WHITE, CYAN, false, 2);
        self.text(432, 208, "VELVET GRID", WHITE, 4);
        self.text(436, 260, "STUDIO", mix(MUTED, WHITE, glow), 2);
        self.rect(340, 376, 280, 6, (37, 28, 44));
        self.rect(
            340,
            376,
            (280.0 * progress.clamp(0.0, 1.0)) as i32,
            6,
            VIOLET,
        );
        self.text(435, 402, "PRESENTA", MUTED, 1);
        self.scanlines(0.08);
    }

    fn consume_event(&mut self, event: &EventView, settings: &Settings) {
        match event.kind.as_str() {
            "pistol" | "shotgun" | "blade" => self.player_shoot = 0.28,
            "dash" => self.player_dash = 0.24,
            "respawn" => self.player_respawn = 0.52,
            _ => {}
        }
        let (count, color, speed, life) = match event.kind.as_str() {
            "pistol" => (8, GOLD, 55.0, 0.22),
            "shotgun" => (18, GOLD, 85.0, 0.3),
            "blade" => (14, CYAN, 70.0, 0.24),
            "deflect" => (16, WHITE, 90.0, 0.3),
            "impact" => (12, RED, 70.0, 0.34),
            "enemy_down" => (24, RED, 105.0, 0.52),
            "player_hit" => (20, RED, 95.0, 0.42),
            "death" => (46, RED, 150.0, 0.9),
            "respawn" => (34, VIOLET, 100.0, 0.72),
            "pickup" => (28, CYAN, 65.0, 0.8),
            "room_clear" => (22, WHITE, 70.0, 0.5),
            "spark" => (5, GOLD, 35.0, 0.18),
            "boss_phase" => (50, VIOLET, 130.0, 0.8),
            "dash" => (8, WHITE, 45.0, 0.16),
            _ => (0, WHITE, 0.0, 0.0),
        };
        if settings.screen_shake
            && matches!(
                event.kind.as_str(),
                "shotgun" | "player_hit" | "death" | "boss_phase"
            )
        {
            self.shake = self.shake.max(event.power.clamp(0.5, 2.0) * 3.0);
        }
        if settings.flashes && matches!(event.kind.as_str(), "death" | "boss_phase") {
            self.flash = 0.42;
        }
        for index in 0..count {
            let angle = index as f32 * 2.399 + self.rand() * 0.5;
            let velocity = speed * (0.45 + self.rand() * 0.65) * event.power.clamp(0.5, 1.8);
            let particle_life = life * (0.65 + self.rand() * 0.5);
            let particle_size = 1.0 + self.rand() * 2.2;
            self.particles.push(Particle {
                pos: event.pos,
                velocity: Vec2 {
                    x: angle.cos() * velocity,
                    y: angle.sin() * velocity,
                },
                life: particle_life,
                max_life: life,
                color,
                size: particle_size,
            });
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    pub fn paint_title(
        &mut self,
        selected: usize,
        can_continue: bool,
        status: &str,
        audio_available: bool,
    ) {
        self.fill(BLACK);
        self.paint_title_background();
        self.text(78, 54, "VELVET GRID STUDIO", MUTED, 1);
        self.text(72, 82, "17", WHITE, 8);
        self.text(76, 172, "MUERE. RECUERDA. ROMPE EL CICLO.", RED, 2);
        self.rect(73, 214, 355, 2, VIOLET);
        let options = [
            "NUEVA PARTIDA",
            "CONTINUAR",
            "COMO JUGAR",
            "AJUSTES",
            "SALIR",
        ];
        for (index, option) in options.iter().enumerate() {
            let y = 246 + index as i32 * 46;
            let disabled = index == 1 && !can_continue;
            if index == selected {
                self.rect(72, y - 9, 350, 34, (38, 24, 47));
                self.rect(72, y - 9, 4, 34, if disabled { MUTED } else { RED });
            }
            let color = if disabled {
                (72, 69, 79)
            } else if index == selected {
                WHITE
            } else {
                MUTED
            };
            self.text(92, y, option, color, 2);
        }
        self.panel(560, 434, 376, 82);
        self.text(580, 448, "ENTER  SELECCIONAR", WHITE, 1);
        self.text(580, 470, "F11    PANTALLA COMPLETA", MUTED, 1);
        self.text(
            580,
            492,
            if audio_available {
                "AUDIO  OK"
            } else {
                "AUDIO  NO DISPONIBLE"
            },
            if audio_available { CYAN } else { RED },
            1,
        );
        if !status.is_empty() {
            let status = self.fit_text(status, 440, 1);
            self.text(76, 498, &status, GOLD, 1);
        }
        self.scanlines(0.08);
    }

    pub fn paint_help(&mut self) {
        self.fill(BLACK);
        self.rect(0, 0, 12, HEIGHT as i32, RED);
        self.text(54, 42, "COMO JUGAR", WHITE, 4);
        self.text(56, 100, "TECLADO Y RATON", RED, 2);
        let keyboard = [
            "WASD / FLECHAS    MOVER",
            "RATON             APUNTAR",
            "CLIC IZQUIERDO    DISPARAR / CORTAR",
            "CLIC DERECHO / SHIFT  IMPULSO",
            "E                 INTERACTUAR",
            "R                 RECARGAR",
            "1 2 3             CAMBIAR ARMA",
            "ESC               PAUSA",
        ];
        for (index, line) in keyboard.iter().enumerate() {
            self.text(56, 142 + index as i32 * 28, line, MUTED, 1);
        }
        self.text(505, 100, "MANDO", VIOLET, 2);
        let gamepad = [
            "STICK IZQ.        MOVER",
            "STICK DER.        APUNTAR",
            "RT / X            ATACAR",
            "LB / B            IMPULSO",
            "A                 INTERACTUAR",
            "Y                 RECARGAR",
            "CRUCETA            CAMBIAR ARMA",
            "START             PAUSA",
        ];
        for (index, line) in gamepad.iter().enumerate() {
            self.text(505, 142 + index as i32 * 28, line, MUTED, 1);
        }
        self.panel(54, 386, 850, 98);
        self.text(75, 405, "1 PISTOLA   2 ESCOPETA   3 HOJA DE FASE", WHITE, 1);
        self.text(
            75,
            431,
            "LA HOJA SOLO ATACA DE CERCA Y TAMBIEN DESVIA BALAS.",
            CYAN,
            1,
        );
        self.text(
            75,
            457,
            "MORIR CAMBIA EL SECTOR; LAS MEMORIAS ABREN OTRO FINAL.",
            MUTED,
            1,
        );
        self.text(56, 506, "ENTER / ESC  VOLVER", MUTED, 1);
    }

    pub fn paint_settings(&mut self, selected: usize, settings: &Settings) {
        self.fill(BLACK);
        self.text(62, 42, "AJUSTES", WHITE, 4);
        self.text(64, 88, "IZQUIERDA / DERECHA PARA CAMBIAR", MUTED, 1);
        let values = [
            (
                "VOLUMEN GENERAL",
                format!("{}%", (settings.master_volume * 100.0) as i32),
            ),
            (
                "MUSICA",
                format!("{}%", (settings.music_volume * 100.0) as i32),
            ),
            (
                "EFECTOS",
                format!("{}%", (settings.effects_volume * 100.0) as i32),
            ),
            ("SACUDIDA", on_off(settings.screen_shake).to_string()),
            ("DISTORSION", on_off(settings.distortion).to_string()),
            ("DESTELLOS", on_off(settings.flashes).to_string()),
            ("ALTO CONTRASTE", on_off(settings.high_contrast).to_string()),
            ("PANTALLA COMPLETA", on_off(settings.fullscreen).to_string()),
            ("BORRAR PARTIDA", String::new()),
            ("VOLVER", String::new()),
        ];
        for (index, (label, value)) in values.iter().enumerate() {
            let y = 120 + index as i32 * 38;
            if index == selected {
                self.rect(60, y - 10, 570, 34, (39, 25, 49));
                self.rect(60, y - 10, 4, 34, VIOLET);
            }
            self.text(
                82,
                y,
                label,
                if index == selected { WHITE } else { MUTED },
                2,
            );
            if !value.is_empty() {
                self.text_right(
                    608,
                    y,
                    value,
                    if index == selected { WHITE } else { MUTED },
                    2,
                );
            }
        }
        self.panel(646, 120, 258, 262);
        self.text(668, 143, "ACCESIBILIDAD", CYAN, 1);
        self.text(668, 178, "REDUCE EFECTOS:", WHITE, 1);
        self.text(680, 202, "- SACUDIDA", MUTED, 1);
        self.text(680, 226, "- DISTORSION", MUTED, 1);
        self.text(680, 250, "- DESTELLOS", MUTED, 1);
        self.text(668, 286, "ALTO CONTRASTE", WHITE, 1);
        self.text(668, 310, "MARCA AMENAZAS", MUTED, 1);
        self.rect(668, 338, 214, 2, STEEL);
        self.text(668, 352, "ENTER  APLICAR", CYAN, 1);
        self.text(62, 506, "ENTER / ESC  VOLVER", MUTED, 1);
    }

    pub fn paint_delete_confirm(&mut self, starts_new: bool) {
        self.fill(BLACK);
        self.paint_title_background();
        self.panel(220, 150, 520, 240);
        self.text(
            270,
            190,
            if starts_new {
                "NUEVA PARTIDA"
            } else {
                "BORRAR PARTIDA"
            },
            WHITE,
            3,
        );
        self.text(270, 245, "SE BORRARA EL PROGRESO ACTUAL.", RED, 2);
        self.text(270, 300, "ENTER  CONFIRMAR", WHITE, 2);
        self.text(270, 338, "ESC    CANCELAR", MUTED, 2);
    }

    pub fn paint_game(&mut self, frame: &FrameView, settings: &Settings) {
        self.fill(if settings.high_contrast {
            (2, 2, 4)
        } else {
            BLACK
        });
        let shake_x = if settings.screen_shake {
            (self.time * 73.0).sin() * self.shake
        } else {
            0.0
        };
        let shake_y = if settings.screen_shake {
            (self.time * 61.0).cos() * self.shake
        } else {
            0.0
        };
        self.paint_room(frame, shake_x, shake_y, settings);
        self.paint_particles(shake_x, shake_y);
        self.paint_hud(frame, settings);

        match frame.phase.as_str() {
            "intro" => self.paint_intro(frame),
            "dead" => self.paint_death(frame, settings),
            "ending_a" | "ending_b" => self.paint_ending(frame),
            "credits" => self.paint_credits(frame),
            "complete" => self.paint_complete(frame),
            _ => {}
        }

        if self.flash > 0.0 && settings.flashes {
            self.tint(WHITE, self.flash.min(0.25));
        }
        if settings.distortion && frame.distortion > 0.05 {
            self.glitch(frame.distortion * if frame.phase == "dead" { 1.0 } else { 0.16 });
        }
        self.scanlines(0.07);
    }

    pub fn paint_pause(&mut self, selected: usize) {
        self.tint(BLACK, 0.72);
        self.panel(270, 112, 420, 318);
        self.text(322, 148, "PAUSA", WHITE, 4);
        let options = ["CONTINUAR", "MEMORIAS", "AJUSTES", "MENU PRINCIPAL"];
        for (index, option) in options.iter().enumerate() {
            let y = 224 + index as i32 * 48;
            if selected == index {
                self.rect(310, y - 9, 330, 34, (49, 26, 48));
                self.rect(310, y - 9, 4, 34, RED);
            }
            self.text(
                334,
                y,
                option,
                if selected == index { WHITE } else { MUTED },
                2,
            );
        }
    }

    pub fn paint_memories(&mut self, frame: &FrameView) {
        self.fill(BLACK);
        self.text(54, 42, "ARCHIVO DE MEMORIAS", WHITE, 4);
        let entries = [
            ("MEMORIA 01", "El sujeto eligio este nombre: Diecisiete."),
            (
                "MEMORIA 02",
                "Cero no era una maquina. Era el primero de nosotros.",
            ),
            (
                "MEMORIA 03",
                "El ciclo no evita la muerte. Alimenta el archivo.",
            ),
        ];
        for (index, (title, body)) in entries.iter().enumerate() {
            let y = 120 + index as i32 * 112;
            self.panel(54, y, 850, 88);
            if frame.memories[index] {
                self.text(76, y + 18, title, CYAN, 2);
                self.text(76, y + 51, body, WHITE, 1);
            } else {
                self.text(76, y + 18, "[DATOS AUSENTES]", (72, 69, 79), 2);
                self.text(
                    76,
                    y + 51,
                    "Busca una fractura luminosa en el Archivo.",
                    MUTED,
                    1,
                );
            }
        }
        self.text(
            56,
            494,
            &format!("RECOBRADAS  {}/3", frame.memory_count),
            GOLD,
            2,
        );
        self.text_right(904, 506, "ENTER / ESC  VOLVER", MUTED, 1);
    }

    fn paint_title_background(&mut self) {
        self.rect(470, 0, 490, 540, (8, 8, 13));
        self.rect(516, 42, 386, 386, (13, 12, 19));
        self.rect(528, 54, 362, 362, OUTLINE);
        for y in (70..414).step_by(32) {
            self.rect(530, y, 358, 2, (32, 24, 38));
        }
        for x in (546..890).step_by(42) {
            self.rect(x, 56, 2, 358, (24, 21, 31));
        }

        // Laboratory gantries and warning lights form a code-drawn backdrop.
        self.rect(546, 86, 24, 286, (27, 29, 37));
        self.rect(551, 94, 5, 270, STEEL);
        self.rect(848, 76, 28, 306, (29, 27, 38));
        self.rect(860, 88, 6, 282, DEEP_VIOLET);
        for y in (104..354).step_by(44) {
            self.rect(542, y, 32, 4, if y % 88 == 16 { RED } else { STEEL_LIGHT });
            self.rect(844, y + 12, 36, 4, if y % 88 == 60 { RED } else { VIOLET });
        }
        self.rect(626, 102, 232, 238, (25, 13, 21));
        self.rect(642, 114, 200, 214, (50, 10, 22));
        for y in (122..324).step_by(18) {
            let shade = if (y / 18) & 1 == 0 {
                BLOOD
            } else {
                (74, 13, 28)
            };
            self.rect(648, y, 188, 3, shade);
        }

        let silhouette = [
            ".........ooooooo.........",
            ".......ooommmmmoo........",
            "......oommmmmmmmmoo......",
            ".....oommmmmmmmmmmoo.....",
            ".....ommmmccccmmmmo......",
            ".....ommmmccccmmmmo......",
            ".....ommmmmmmmmmmoo......",
            "......oommmmmmmmoo.......",
            ".......ooommmmooo........",
            "........ommmmmo..........",
            "......ooommmmmmooo........",
            "...oooommmmmmmmmmooooo....",
            "..ommmmmmmmmmmmmmmmmmmmo..",
            "..ommmmmlmmmmmmmllllllao..",
            "ooommmmmmmmmmmmmmmmooooo..",
            "olammmmmmmmmmmmmmmmo......",
            "ooommmmmmmmmmmmmmmmmo.....",
            "..oommmmmmmmmmmmmmmmo.....",
            "...ommmmmmmmmmmmmmmmo.....",
            "...ommmmmmmmmmmmmmmmo.....",
            "....ommmmmmmmmmmmmmo......",
            "....oommmmmmmmmmmmoo......",
            ".....ommmmmoommmmmo.......",
            ".....ommmmmo.ommmmmo......",
            "....oommmmmo.oommmmmo.....",
            "....ommmmmoo..ommmmmo.....",
            "...oommmmmo...oommmmmo....",
            "...ooooooo.....ooooooo....",
        ];
        if !self.paint_bot_portrait(377, 132, 0, false, 4) {
            self.blit_sprite_scaled(377, 132, &silhouette, (31, 37, 48), STEEL, CYAN, false, 3);
        }

        self.rect(520, 426, 380, 4, RED);
        for x in (540..900).step_by(48) {
            self.line(710, 430, x, 540, (37, 25, 43));
        }
        for y in (442..540).step_by(18) {
            self.rect(500, y, 430, 2, (31, 24, 37));
        }
        let blink = ((self.time * 3.0) as i32) & 1 == 0;
        self.rect(874, 42, 12, 6, if blink { RED } else { BLOOD });
        self.text(
            594,
            404,
            if self.art.neo_zero_loaded() {
                self.art.status_label()
            } else {
                "SUJETO 17 // SINCRONIZADO"
            },
            if blink { CYAN } else { STEEL_LIGHT },
            1,
        );
    }

    fn paint_room(&mut self, frame: &FrameView, ox: f32, oy: f32, settings: &Settings) {
        self.paint_floor(frame, ox, oy, settings);
        self.world_rect(8.0, 8.0, 624.0, 8.0, (52, 40, 57), ox, oy);
        self.world_rect(8.0, 344.0, 624.0, 8.0, (29, 22, 35), ox, oy);
        self.world_rect(8.0, 8.0, 8.0, 344.0, (47, 35, 52), ox, oy);
        self.world_rect(624.0, 8.0, 8.0, 344.0, (27, 20, 32), ox, oy);
        for x in (20..620).step_by(32) {
            self.world_rect(x as f32, 10.0, 12.0, 2.0, STEEL, ox, oy);
            self.world_rect(x as f32, 347.0, 7.0, 1.0, BLOOD, ox, oy);
        }

        for hazard in &frame.hazards {
            self.world_rect(hazard.x, hazard.y, hazard.w, hazard.h, (62, 10, 22), ox, oy);
            let pulse = 0.5 + (self.time * 5.0).sin() * 0.5;
            self.world_rect(
                hazard.x + 3.0,
                hazard.y + 3.0,
                hazard.w - 6.0,
                3.0,
                mix((92, 17, 32), RED, pulse),
                ox,
                oy,
            );
            let mut stripe = 4.0;
            while stripe < hazard.w {
                self.world_rect(
                    hazard.x + stripe,
                    hazard.y + hazard.h - 5.0,
                    5.0,
                    2.0,
                    GOLD,
                    ox,
                    oy,
                );
                stripe += 12.0;
            }
        }
        for obstacle in &frame.obstacles {
            self.paint_obstacle(obstacle, ox, oy);
        }
        if frame.door_open {
            let pulse = if (self.time * 5.0).sin() > 0.0 {
                CYAN
            } else {
                WHITE
            };
            self.world_rect(612.0, 144.0, 13.0, 72.0, (18, 38, 43), ox, oy);
            self.world_rect(616.0, 150.0, 7.0, 60.0, pulse, ox, oy);
            self.world_rect(609.0, 174.0, 5.0, 12.0, WHITE, ox, oy);
        } else {
            self.world_rect(612.0, 144.0, 13.0, 72.0, OUTLINE, ox, oy);
            self.world_rect(616.0, 150.0, 7.0, 60.0, BLOOD, ox, oy);
            for y in (154..208).step_by(10) {
                self.world_rect(616.0, y as f32, 7.0, 3.0, STEEL, ox, oy);
            }
        }
        if frame.phase == "choice" {
            self.paint_core(Vec2 { x: 205.0, y: 180.0 }, RED, ox, oy);
            self.paint_core(Vec2 { x: 435.0, y: 180.0 }, CYAN, ox, oy);
        }
        for pickup in &frame.pickups {
            if !pickup.active {
                continue;
            }
            self.paint_pickup(pickup, ox, oy);
        }
        for bullet in &frame.bullets {
            if !bullet.alive {
                continue;
            }
            let color = if bullet.owner == "player" { GOLD } else { RED };
            let tail = Vec2 {
                x: bullet.pos.x - bullet.velocity.x * 0.018,
                y: bullet.pos.y - bullet.velocity.y * 0.018,
            };
            self.world_line(tail, bullet.pos, color, ox, oy);
            let p = self.world_raw(bullet.pos, ox, oy);
            let radius = ((bullet.radius * SCALE / PIXEL_SCALE as f32).round() as i32).clamp(1, 2);
            self.rect_raw(
                p.0 - radius,
                p.1 - radius,
                radius * 2 + 1,
                radius * 2 + 1,
                WHITE,
            );
            self.put_raw(p.0, p.1, color);
        }
        for enemy in &frame.enemies {
            if enemy.alive {
                self.paint_enemy(enemy, ox, oy, settings);
            }
        }
        if frame.player.hp > 0.0 || frame.phase == "dead" {
            self.paint_player(frame, ox, oy);
        }
    }

    fn paint_floor(&mut self, frame: &FrameView, ox: f32, oy: f32, settings: &Settings) {
        let (base, seam, accent) = match frame.room {
            1 => ((17, 22, 27), (31, 40, 45), (35, 63, 65)),
            2 => ((22, 18, 26), (43, 30, 42), (70, 29, 43)),
            3 => ((18, 17, 25), (37, 31, 47), (49, 34, 70)),
            4 => ((24, 15, 21), (53, 27, 35), (92, 25, 39)),
            5 => ((16, 14, 24), (38, 28, 52), (61, 37, 83)),
            _ => ((17, 18, 24), (34, 34, 43), (48, 40, 58)),
        };
        self.world_rect(8.0, 8.0, 624.0, 344.0, base, ox, oy);

        for x in (16..632).step_by(32) {
            self.world_rect(x as f32, 12.0, 1.0, 336.0, seam, ox, oy);
        }
        for y in (16..352).step_by(32) {
            self.world_rect(12.0, y as f32, 616.0, 1.0, seam, ox, oy);
        }
        for y in (30..338).step_by(64) {
            for x in (30..620).step_by(64) {
                self.world_rect(x as f32, y as f32, 2.0, 2.0, accent, ox, oy);
                if (x + y + frame.room as i32) % 3 == 0 {
                    self.world_rect(x as f32 + 8.0, y as f32 + 8.0, 9.0, 1.0, seam, ox, oy);
                    self.world_rect(x as f32 + 16.0, y as f32 + 8.0, 1.0, 6.0, seam, ox, oy);
                }
            }
        }

        // Light is quantized through a checker pattern so the room remains pixel art.
        let player = self.world_raw(frame.player.pos, ox, oy);
        let radius = if settings.high_contrast { 130.0 } else { 96.0 };
        for y in 6..(PIXEL_HEIGHT as i32 - 6) {
            for x in 6..(PIXEL_WIDTH as i32 - 6) {
                let dx = (x - player.0) as f32;
                let dy = (y - player.1) as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance <= radius * 0.48 {
                    continue;
                }
                let darkness = ((distance - radius * 0.48) / (radius * 0.66)).clamp(0.0, 0.72);
                let threshold = match (x & 1, y & 1) {
                    (0, 0) => 0.18,
                    (1, 1) => 0.42,
                    _ => 0.66,
                };
                if darkness > threshold {
                    let index = (y as u32 * PIXEL_WIDTH + x as u32) as usize;
                    let pixel = self.pixels[index];
                    let color = (
                        ((pixel >> 16) & 255) as u8,
                        ((pixel >> 8) & 255) as u8,
                        (pixel & 255) as u8,
                    );
                    let target = if darkness > 0.62 { BLACK } else { INK };
                    self.pixels[index] = {
                        let shaded = mix(color, target, 0.52 + darkness * 0.32);
                        pack_rgb(shaded.0, shaded.1, shaded.2)
                    };
                }
            }
        }
    }

    fn paint_obstacle(&mut self, obstacle: &RectView, ox: f32, oy: f32) {
        let x = obstacle.x;
        let y = obstacle.y;
        let w = obstacle.w;
        let h = obstacle.h;
        self.world_rect(x + 3.0, y + 5.0, w, h, OUTLINE, ox, oy);
        match obstacle.kind.as_str() {
            "containment" => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, STEEL, ox, oy);
                self.world_rect(x + 7.0, y + 7.0, w - 14.0, h - 14.0, (17, 42, 48), ox, oy);
                self.world_rect(x + 9.0, y + 9.0, 3.0, h - 18.0, CYAN, ox, oy);
                self.world_rect(x + w - 13.0, y + 9.0, 2.0, h - 18.0, STEEL_LIGHT, ox, oy);
                for sy in (12..h.max(13.0) as i32).step_by(18) {
                    self.world_rect(x + 1.0, y + sy as f32, w - 2.0, 3.0, OUTLINE, ox, oy);
                }
            }
            "terminal" => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, STEEL, ox, oy);
                self.world_rect(x + 8.0, y + 6.0, w - 26.0, h - 13.0, (9, 34, 38), ox, oy);
                self.world_rect(x + 11.0, y + 9.0, w - 34.0, 2.0, CYAN, ox, oy);
                self.world_rect(x + w - 13.0, y + 7.0, 4.0, 4.0, RED, ox, oy);
                self.world_rect(x + w - 13.0, y + 15.0, 4.0, 3.0, GOLD, ox, oy);
            }
            "archive" => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, (38, 31, 47), ox, oy);
                for sy in (9..h.max(10.0) as i32).step_by(15) {
                    self.world_rect(x + 5.0, y + sy as f32, w - 10.0, 3.0, STEEL_LIGHT, ox, oy);
                    self.world_rect(x + 8.0, y + sy as f32 - 4.0, 3.0, 3.0, VIOLET, ox, oy);
                    self.world_rect(x + 15.0, y + sy as f32 - 4.0, 7.0, 3.0, CYAN, ox, oy);
                }
            }
            "purge" => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, (55, 28, 35), ox, oy);
                self.world_rect(x + 7.0, y + 7.0, w - 14.0, h - 14.0, STEEL, ox, oy);
                for sx in (8..w.max(9.0) as i32).step_by(16) {
                    self.world_rect(x + sx as f32, y + 3.0, 7.0, 4.0, GOLD, ox, oy);
                }
                self.world_rect(x + 8.0, y + h - 12.0, w - 16.0, 4.0, RED, ox, oy);
            }
            "zero" => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, DEEP_VIOLET, ox, oy);
                self.world_rect(x + w * 0.5 - 2.0, y + 7.0, 4.0, h - 14.0, VIOLET, ox, oy);
                for sy in (11..h.max(12.0) as i32).step_by(19) {
                    self.world_rect(
                        x + 8.0,
                        y + sy as f32,
                        w - 16.0,
                        2.0,
                        (161, 98, 218),
                        ox,
                        oy,
                    );
                }
            }
            _ => {
                self.world_rect(x, y, w, h, OUTLINE, ox, oy);
                self.world_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, STEEL, ox, oy);
                self.world_line(
                    Vec2 {
                        x: x + 5.0,
                        y: y + 5.0,
                    },
                    Vec2 {
                        x: x + w - 5.0,
                        y: y + h - 5.0,
                    },
                    STEEL_LIGHT,
                    ox,
                    oy,
                );
                self.world_line(
                    Vec2 {
                        x: x + w - 5.0,
                        y: y + 5.0,
                    },
                    Vec2 {
                        x: x + 5.0,
                        y: y + h - 5.0,
                    },
                    STEEL_LIGHT,
                    ox,
                    oy,
                );
                self.world_rect(x + 6.0, y + 6.0, 3.0, 3.0, GOLD, ox, oy);
                self.world_rect(x + w - 9.0, y + h - 9.0, 3.0, 3.0, GOLD, ox, oy);
            }
        }
    }

    fn paint_core(&mut self, pos: Vec2, color: (u8, u8, u8), ox: f32, oy: f32) {
        let (x, y) = self.world_raw(pos, ox, oy);
        self.shadow_raw(x, y + 12, 12);
        self.rect_raw(x - 9, y + 7, 19, 5, OUTLINE);
        self.rect_raw(x - 7, y + 6, 15, 4, STEEL);
        let pattern = [
            "....ooooo....",
            "..ooomomooo..",
            ".oommmlmmmoo.",
            "oommlaolmmmoo",
            "ommlaaallmmmo",
            "omlaawaaalmmo",
            "ommlaaallmmmo",
            "oommlaolmmmoo",
            ".oommmlmmmoo.",
            "..ooomomooo..",
            "....ooooo....",
        ];
        self.blit_sprite(x, y, &pattern, color, mix(color, WHITE, 0.55), WHITE, false);
        if (self.time * 5.0).sin() > 0.0 {
            self.put_raw(x, y - 7, color);
            self.put_raw(x - 12, y, color);
            self.put_raw(x + 12, y, color);
        }
    }

    fn paint_pickup(&mut self, pickup: &PickupView, ox: f32, oy: f32) {
        let (x, mut y) = self.world_raw(pickup.pos, ox, oy);
        y += (self.time * 3.0 + pickup.index as f32).sin().round() as i32;
        let pulse = 0.5 + (self.time * 4.0 + pickup.pulse).sin() * 0.5;
        let color = if pickup.kind == "memory" { CYAN } else { GOLD };
        self.shadow_raw(x, y + 8, 9);
        let pattern: &[&str] = match pickup.kind.as_str() {
            "shotgun" => &[
                "...........oo",
                "..ooooooooomm",
                ".ollllllllmmo.",
                "ooommmmoooo...",
                "...oomo.......",
                "....oo........",
            ],
            "blade" => &[
                ".........wl",
                ".......wll.",
                ".....wll...",
                "...wll.....",
                ".owl.......",
                "oo.........",
                "mo.........",
            ],
            "memory" => &[
                "...ooo...",
                ".ooaca...",
                "oacwwcao.",
                "ocwaaaco.",
                "oacwwcao.",
                ".ooaca...",
                "...ooo...",
            ],
            _ => &[
                "..ooooooo",
                ".olllllmo",
                "oommmmmoo",
                "...omo...",
                "...ooo...",
            ],
        };
        self.blit_sprite(x, y, pattern, color, mix(color, WHITE, 0.55), CYAN, false);
        if pulse > 0.55 {
            self.put_raw(x - 10, y - 7, color);
            self.put_raw(x + 10, y + 5, color);
            self.put_raw(x + 8, y - 9, WHITE);
        }
    }

    fn paint_player(&mut self, frame: &FrameView, ox: f32, oy: f32) {
        let player = &frame.player;
        let (x, mut y) = self.world_raw(player.pos, ox, oy);
        let moving = player.velocity.x.abs() + player.velocity.y.abs() > 8.0;
        let frame_step = ((self.time * 9.0) as i32) & 1;
        if moving && frame_step == 1 {
            y -= 1;
        }
        let flip = player.aim.x < 0.0;
        let main = if player.hit_flash > 0.0 {
            WHITE
        } else if player.invulnerable > 0.0 && frame_step == 1 {
            VIOLET
        } else {
            (45, 151, 158)
        };
        let accent = if frame.player.weapon == "blade" {
            VIOLET
        } else {
            GOLD
        };
        self.shadow_raw(x, y + 8, 8);
        if self.paint_bot_player(frame, x, y) {
            if frame.phase != "dead" {
                let muzzle = Vec2 {
                    x: player.pos.x + player.aim.x * 15.0,
                    y: player.pos.y + player.aim.y * 15.0,
                };
                let m = self.world_raw(muzzle, ox, oy);
                self.put_raw(m.0, m.1, WHITE);
            }
            return;
        }
        if frame.phase == "dead" {
            return;
        }
        let effect = if player.hit_flash > 0.0 {
            Some((WHITE, 0.88))
        } else if player.invulnerable > 0.0 && frame_step == 1 {
            Some((VIOLET, 0.58))
        } else {
            Some((CYAN, 0.52))
        };
        if self.paint_neo_character(x, y, 4, player.aim, moving, effect, 1.0) {
            let muzzle = Vec2 {
                x: player.pos.x + player.aim.x * 15.0,
                y: player.pos.y + player.aim.y * 15.0,
            };
            let m = self.world_raw(muzzle, ox, oy);
            self.put_raw(m.0, m.1, WHITE);
            return;
        }
        let pattern = if moving && frame_step == 1 {
            [
                ".....ooo.....",
                "....ommo.....",
                "...omccmo....",
                "...omccmo....",
                "....ooooo....",
                "...ommmmmooo.",
                "..oommlmmolla",
                ".oommmmmmooo..",
                "...ommmmmo....",
                "....ommo.....",
                "...oo..oo....",
                "..oo....oo...",
            ]
        } else {
            [
                ".....ooo.....",
                "....ommo.....",
                "...omccmo....",
                "...omccmo....",
                "....ooooo....",
                "...ommmmmooo.",
                "..oommlmmolla",
                ".oommmmmmooo..",
                "...ommmmmo....",
                "....ommo.....",
                "...oo.oo.....",
                "...oo..oo....",
            ]
        };
        self.blit_sprite(x, y, &pattern, main, mix(main, WHITE, 0.45), accent, flip);
        // Aiming remains visible in every stance and is not encoded by color alone.
        let muzzle = Vec2 {
            x: player.pos.x + player.aim.x * 15.0,
            y: player.pos.y + player.aim.y * 15.0,
        };
        let m = self.world_raw(muzzle, ox, oy);
        self.put_raw(m.0, m.1, WHITE);
    }

    fn paint_enemy(&mut self, enemy: &EnemyView, ox: f32, oy: f32, settings: &Settings) {
        let mut color = if enemy.hit_flash > 0.0 { WHITE } else { RED };
        if settings.high_contrast && enemy.hit_flash <= 0.0 {
            color = (255, 67, 82);
        }
        let (x, mut y) = self.world_raw(enemy.pos, ox, oy);
        let flip = enemy.aim.x < 0.0;
        let step = ((self.time * 7.0 + enemy.pos.x * 0.03) as i32) & 1;
        if step == 1 && enemy.kind != "zero" {
            y -= 1;
        }
        self.shadow_raw(x, y + 9, if enemy.kind == "zero" { 14 } else { 9 });
        let external_effect = if enemy.hit_flash > 0.0 {
            Some((WHITE, 0.9))
        } else if settings.high_contrast {
            Some((WHITE, 0.22))
        } else {
            Some((RED, 0.44))
        };
        let external = match enemy.kind.as_str() {
            "vigilante" => self.paint_neo_character(x, y, 0, enemy.aim, true, external_effect, 1.0),
            "echo" | "echo_clone" => {
                let loaded = self.paint_neo_character(
                    x - 2,
                    y + 1,
                    8,
                    enemy.aim,
                    true,
                    Some((VIOLET, 0.72)),
                    0.38,
                );
                if loaded {
                    self.paint_neo_character(
                        x,
                        y - 1,
                        8,
                        enemy.aim,
                        true,
                        external_effect.or(Some((VIOLET, 0.28))),
                        0.96,
                    );
                }
                loaded
            }
            _ => false,
        };
        if !external {
            match enemy.kind.as_str() {
                "vigilante" => {
                    let sprite = [
                        "....ooooo....",
                        "...ommmmmo...",
                        "..omlrrlmmo..",
                        "..omlrrlmmo..",
                        ".ooommmmmoooo",
                        "ommmmmmmmolaa",
                        "ommlmmmmlmooo",
                        "ommmmmmmmmmo.",
                        ".oommmmmmmmoo",
                        "...ommmmmo...",
                        "..oo.ooo.oo..",
                        ".oo.......oo.",
                    ];
                    self.blit_sprite(x, y, &sprite, color, STEEL_LIGHT, GOLD, flip);
                }
                "hound" => {
                    let sprite = if step == 0 {
                        [
                            "........oooo...",
                            "..oo..oommmmo..",
                            ".ommoommlrrmoo..",
                            "ommmmmmmmmmmmoo",
                            "ooommmmmmmmooo.",
                            "..oo.oo..oo....",
                            ".oo...oo...oo...",
                        ]
                    } else {
                        [
                            "........oooo...",
                            "..oo..oommmmo..",
                            ".ommoommlrrmoo..",
                            "ommmmmmmmmmmmoo",
                            "ooommmmmmmmooo.",
                            ".oo...oo.oo.....",
                            "..oo.oo...oo....",
                        ]
                    };
                    self.blit_sprite(x, y, &sprite, color, STEEL_LIGHT, GOLD, flip);
                }
                "echo" | "echo_clone" => {
                    let ghost = (104, 65, 143);
                    let sprite = [
                        ".....ooo.....",
                        "....ommo.....",
                        "...omvvmo....",
                        "...omvvmo....",
                        "....ooooo....",
                        "...ommmmmooo.",
                        "..oommlmmovva",
                        ".oommmmmmooo..",
                        "...ommmmmo....",
                        "....ommo.....",
                        "...oo.oo.....",
                        "..oo...oo....",
                    ];
                    self.blit_sprite(x - 3, y, &sprite, ghost, VIOLET, WHITE, flip);
                    self.blit_sprite(x, y - 1, &sprite, VIOLET, (190, 124, 231), CYAN, flip);
                }
                "zero" => {
                    let sprite = [
                        ".......ooo.......",
                        ".....oommmoo.....",
                        "....omvvvvmo....",
                        "...omvvwwvvmo...",
                        "...omvvvvvvmo...",
                        "....oommmmoo....",
                        "..ooommmmmmooo..",
                        ".ommmmmmmmmmmmmo.",
                        "ommmmlmmmmmlmmmmo",
                        "ommmmmmmmmmmmmmmmo",
                        "oommmmmmmmmmmmmmoo",
                        ".ommmmmmmmmmmmmo.",
                        "..ommmmmmmmmmo..",
                        "..ommmmmmmmmmo..",
                        ".ommmmmoommmmmo.",
                        "ommmmoo..oommmmo",
                        "ooooo......ooooo",
                    ];
                    let light = if enemy.phase > 1 {
                        WHITE
                    } else {
                        (178, 105, 229)
                    };
                    self.blit_sprite(x, y, &sprite, color, light, VIOLET, flip);
                    if step == 1 {
                        self.put_raw(x - 12, y - 8, VIOLET);
                        self.put_raw(x + 12, y - 3, VIOLET);
                        self.put_raw(x + 10, y + 8, WHITE);
                    }
                }
                _ => {
                    let sprite = ["..ooo..", ".omrmo.", "ommmmmo", ".omlmo.", ".oo.oo."];
                    self.blit_sprite(x, y, &sprite, color, STEEL_LIGHT, GOLD, flip);
                }
            }
        }
        let ratio = if enemy.max_hp > 0.0 {
            (enemy.hp / enemy.max_hp).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.world_rect(
            enemy.pos.x - 12.0,
            enemy.pos.y - 18.0,
            24.0,
            2.0,
            (46, 34, 43),
            ox,
            oy,
        );
        self.world_rect(
            enemy.pos.x - 12.0,
            enemy.pos.y - 18.0,
            24.0 * ratio,
            2.0,
            RED,
            ox,
            oy,
        );
    }

    fn paint_particles(&mut self, ox: f32, oy: f32) {
        let particles = self.particles.clone();
        for particle in particles {
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let (x, y) = self.world_raw(particle.pos, ox, oy);
            let size = (particle.size * alpha).ceil().max(1.0) as i32;
            self.rect_raw(
                x - size / 2,
                y - size / 2,
                size,
                size,
                mix(BLACK, particle.color, alpha),
            );
            if size > 1 {
                let tail = Vec2 {
                    x: particle.pos.x - particle.velocity.x * 0.018,
                    y: particle.pos.y - particle.velocity.y * 0.018,
                };
                let t = self.world_raw(tail, ox, oy);
                self.line_raw(x, y, t.0, t.1, mix(BLACK, particle.color, alpha * 0.7));
            }
        }
    }

    fn paint_hud(&mut self, frame: &FrameView, settings: &Settings) {
        self.panel(24, 24, 336, 98);
        self.text(40, 38, "17", WHITE, 3);
        let room_name = self.fit_text(&frame.room_name, 232, 1);
        self.text(104, 38, &room_name, WHITE, 1);
        self.text(
            104,
            58,
            &format!(
                "SECTOR {:02} // {:02}:{:02}",
                frame.room,
                frame.room_time as i32 / 60,
                frame.room_time as i32 % 60
            ),
            MUTED,
            1,
        );
        let hp = if frame.player.max_hp > 0.0 {
            (frame.player.hp / frame.player.max_hp).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.text(
            104,
            80,
            &format!(
                "VIDA {:03}/{:03}",
                frame.player.hp.max(0.0) as i32,
                frame.player.max_hp.max(0.0) as i32
            ),
            WHITE,
            1,
        );
        self.rect(104, 104, 232, 8, (44, 30, 38));
        self.rect(
            104,
            104,
            (232.0 * hp) as i32,
            8,
            if settings.high_contrast { WHITE } else { RED },
        );

        self.panel(690, 24, 246, 98);
        let (weapon, descriptor) = match frame.player.weapon.as_str() {
            "pistol" => ("PISTOLA", "DISPARO PRECISO"),
            "shotgun" => ("ESCOPETA", "DISPERSION CORTA"),
            "blade" => ("HOJA", "CUERPO A CUERPO"),
            _ => ("SIN ARMA", "BUSCA EQUIPO"),
        };
        self.text(708, 38, weapon, GOLD, 2);
        let ammo = if frame.player.weapon == "blade" {
            "INFINITA".to_string()
        } else {
            format!("{:02}/{:02}", frame.ammo, frame.magazine)
        };
        self.text_right(918, 38, &ammo, WHITE, 1);
        self.text(708, 59, descriptor, MUTED, 1);
        self.rect(708, 78, 210, 2, STEEL);
        self.text(708, 86, &format!("PUNTOS {:06}", frame.score), MUTED, 1);
        if frame.player.reload_timer > 0.0 {
            self.text_right(918, 104, "RECARGANDO", RED, 1);
        } else if frame.player.dash_cooldown <= 0.0 {
            self.text_right(918, 104, "IMPULSO OK", CYAN, 1);
        } else {
            self.text_right(918, 104, "IMPULSO --", MUTED, 1);
        }

        let alive = frame.enemies.iter().filter(|enemy| enemy.alive).count();
        if frame.boss_max > 0.0 && frame.boss_hp > 0.0 {
            self.panel(370, 24, 300, 50);
            self.text(388, 36, "CERO", WHITE, 1);
            self.rect(450, 39, 198, 8, (45, 28, 45));
            self.rect(
                450,
                39,
                (198.0 * frame.boss_hp / frame.boss_max) as i32,
                8,
                VIOLET,
            );
            self.text_centered(370, 300, 57, "JEFE // FASE ACTIVA", MUTED, 1);
        } else {
            let objective = if frame.room_clear {
                "RUTA ABIERTA  >".to_string()
            } else {
                match frame.room {
                    1 => "OBJETIVO  ENCUENTRA LA SALIDA".to_string(),
                    2 | 4 => format!("OBJETIVO  AMENAZAS {alive:02}"),
                    3 => format!("MEMORIAS {}/3  //  EXPLORA", frame.memory_count),
                    5 => "OBJETIVO  DERROTA A CERO".to_string(),
                    _ => "OBJETIVO  AVANZA".to_string(),
                }
            };
            self.panel(370, 24, 300, 42);
            self.text_centered(
                380,
                280,
                38,
                &objective,
                if frame.room_clear { CYAN } else { WHITE },
                1,
            );
        }

        self.panel(24, 450, 198, 66);
        self.text(40, 466, &format!("MUERTES  {:02}", frame.deaths), RED, 2);
        self.text(
            40,
            492,
            &format!("MEMORIAS {}/3", frame.memory_count),
            CYAN,
            1,
        );
        if !frame.message.is_empty() {
            self.panel(230, 414, 500, 102);
            let speaker = self.fit_text(&frame.speaker, 452, 1);
            self.text(252, 432, &speaker, RED, 1);
            self.text_wrapped(252, 456, 456, &frame.message, WHITE, 1);
        } else if !frame.prompt.is_empty() {
            let width = (self.text_width(&frame.prompt, 1) + 48).clamp(300, 680);
            let x = (WIDTH as i32 - width) / 2;
            self.panel(x, 452, width, 54);
            let prompt = self.fit_text(&frame.prompt, width - 40, 1);
            self.text_centered(x + 20, width - 40, 470, &prompt, WHITE, 1);
        }
        if !frame.memory_text.is_empty() {
            self.tint(BLACK, 0.8);
            self.panel(128, 112, 704, 316);
            self.text(168, 152, "MEMORIA RECUPERADA", CYAN, 3);
            self.text_wrapped(168, 220, 624, &frame.memory_text, WHITE, 2);
            self.text(168, 378, "E / A  CERRAR", WHITE, 1);
        }
    }

    fn paint_intro(&mut self, frame: &FrameView) {
        self.tint(BLACK, 0.88);
        let phase = (self.time * 0.7).fract();
        self.panel(218, 90, 524, 352);
        for x in (238..722).step_by(24) {
            self.rect(x, 110, 2, 252, (31, 24, 38));
        }
        for y in (110..362).step_by(24) {
            self.rect(238, y, 484, 2, (31, 24, 38));
        }
        self.rect(500, 110, 222, 252, (10, 9, 15));
        let scan = [
            ".....ooooo.....",
            "...ooommmooo...",
            "..oommmmmmmmoo..",
            "..ommmcccmmmmo..",
            "..ommmcccmmmmo..",
            "...oommmmmmoo...",
            "....oommmoo.....",
            ".....ommmo......",
            "...ooommmmmooo..",
            ".oommmmmmmmmmmmoo",
            "ommmmmmmmmmmmmmo",
            "oommmmmmmmmmmmmo",
            "..ommmmmmmmmmmo.",
            "...ommmmmmmmmo..",
            "...ommmmoommmmo.",
            "..oommmmo.ommmoo",
            "..ommmmoo.oommmo",
            ".oommmmo...ommmmo",
            ".oooooo.....ooooo",
        ];
        if !self.paint_bot_portrait(306, 118, ((frame.room_time * 7.0) as u32).min(4), true, 3) {
            self.blit_sprite_scaled(306, 118, &scan, (35, 47, 54), STEEL_LIGHT, CYAN, false, 2);
        }
        self.rect(
            504,
            116 + (phase * 236.0) as i32,
            214,
            4,
            mix(CYAN, WHITE, phase),
        );
        self.text(252, 126, "SUJETO", MUTED, 1);
        self.text(248, 160, "17", WHITE, 7);
        self.text(248, 258, "ESTADO", MUTED, 1);
        self.text(248, 282, "RECUPERADO", CYAN, 2);
        self.text(266, 382, "RECONSTRUCCION NEURAL COMPLETA", RED, 2);
        self.text(376, 414, "E / A  DESPERTAR", WHITE, 1);
    }

    fn paint_death(&mut self, frame: &FrameView, _: &Settings) {
        self.tint((45, 0, 8), 0.62);
        self.panel(278, 136, 404, 270);
        self.text_centered(302, 356, 174, "MUERTE", WHITE, 6);
        self.text_centered(
            302,
            356,
            252,
            &format!("REGISTRO {:02}", frame.deaths),
            RED,
            3,
        );
        self.text_centered(302, 356, 318, "EL MUNDO ESTA APRENDIENDO", MUTED, 1);
        self.rect(322, 360, 316, 6, (50, 21, 30));
        self.rect(
            322,
            360,
            (316.0 * (1.0 - frame.death_timer / 1.35).clamp(0.0, 1.0)) as i32,
            6,
            RED,
        );
    }

    fn paint_ending(&mut self, frame: &FrameView) {
        self.tint(BLACK, 0.94);
        self.panel(112, 82, 736, 376);
        let (label, title, body, color) = if frame.phase == "ending_a" {
            (
                "FINAL I",
                "ROMPER EL CICLO",
                "Diecisiete destruye el nucleo. Por primera vez, la proxima muerte sera la ultima.",
                RED,
            )
        } else {
            (
                "FINAL II",
                "RECORDAR A TODOS",
                "Diecisiete abre el archivo. Dieciseis vidas regresan y Cero deja de estar solo.",
                CYAN,
            )
        };
        self.rect(112, 82, 8, 376, color);
        self.text_centered(144, 672, 120, label, color, 3);
        self.text_centered(144, 672, 176, title, WHITE, 4);
        self.text_wrapped(160, 250, 640, body, WHITE, 2);
        self.panel(160, 330, 640, 52);
        let variant = format!("VARIANTE // {}", frame.ending_variant.to_uppercase());
        self.text_centered(180, 600, 348, &variant, GOLD, 1);
        self.text_centered(160, 640, 416, "E / A  CONTINUAR", MUTED, 1);
    }

    fn paint_credits(&mut self, frame: &FrameView) {
        self.fill(BLACK);
        self.text_centered(24, 912, 56, "17", WHITE, 7);
        self.text_centered(24, 912, 158, "UN JUEGO DE VELVET GRID STUDIO", RED, 2);
        self.text_centered(24, 912, 224, "HECHO CON", MUTED, 1);
        self.text_centered(24, 912, 260, "VELVETENGINE + VS3", CYAN, 3);
        if self.art.neo_zero_loaded() {
            self.text_centered(24, 912, 298, "ARTE ADICIONAL  YANIN / NEO ZERO", GOLD, 1);
        }
        if self.art.bot_wheel_loaded() {
            self.text_centered(
                24,
                912,
                320,
                "BOT WHEEL  ASSET LOCAL / LICENCIA PENDIENTE",
                VIOLET,
                1,
            );
        }
        self.panel(300, 348, 360, 100);
        self.text(
            328,
            356,
            &format!("MUERTES     {:02}", frame.deaths),
            WHITE,
            2,
        );
        self.text(
            328,
            392,
            &format!("PUNTUACION  {:06}", frame.score),
            WHITE,
            2,
        );
        self.text_centered(24, 912, 474, "E / A  TERMINAR", MUTED, 1);
    }

    fn paint_complete(&mut self, frame: &FrameView) {
        self.fill(BLACK);
        self.panel(252, 82, 456, 366);
        self.text_centered(276, 408, 118, "CICLO CERRADO", WHITE, 4);
        self.rect(300, 174, 360, 2, STEEL);
        self.text(
            324,
            206,
            &format!("PUNTUACION  {:06}", frame.score),
            GOLD,
            2,
        );
        self.text(
            324,
            248,
            &format!("MUERTES     {:02}", frame.deaths),
            RED,
            2,
        );
        self.text(
            324,
            290,
            &format!("MEMORIAS    {}/3", frame.memory_count),
            CYAN,
            2,
        );
        self.text_centered(276, 408, 382, "ENTER  MENU PRINCIPAL", WHITE, 1);
    }

    fn fill(&mut self, color: (u8, u8, u8)) {
        self.pixels.fill(pack_rgb(color.0, color.1, color.2));
    }

    fn panel(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect(x, y, w, h, OUTLINE);
        self.rect(x + 2, y, w - 4, 2, (74, 62, 83));
        self.rect(x, y + 2, 2, h - 4, (74, 62, 83));
        self.rect(x + 2, y + 2, w - 4, h - 4, PANEL);
        self.rect(x, y, 4, 4, BLACK);
        self.rect(x + w - 4, y, 4, 4, BLACK);
        self.rect(x, y + h - 4, 4, 4, BLACK);
        self.rect(x + w - 4, y + h - 4, 4, 4, BLACK);
        self.rect(x + 8, y + 4, 20, 2, DEEP_VIOLET);
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: (u8, u8, u8)) {
        if w <= 0 || h <= 0 {
            return;
        }
        let x = x.div_euclid(PIXEL_SCALE);
        let y = y.div_euclid(PIXEL_SCALE);
        let w = ((w + PIXEL_SCALE - 1) / PIXEL_SCALE).max(1);
        let h = ((h + PIXEL_SCALE - 1) / PIXEL_SCALE).max(1);
        self.rect_raw(x, y, w, h, color);
    }

    fn rect_raw(&mut self, x: i32, y: i32, w: i32, h: i32, color: (u8, u8, u8)) {
        let packed = pack_rgb(color.0, color.1, color.2);
        for py in y.max(0)..(y + h).min(PIXEL_HEIGHT as i32) {
            for px in x.max(0)..(x + w).min(PIXEL_WIDTH as i32) {
                self.pixels[(py as u32 * PIXEL_WIDTH + px as u32) as usize] = packed;
            }
        }
    }

    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: (u8, u8, u8)) {
        x0 = x0.div_euclid(PIXEL_SCALE);
        y0 = y0.div_euclid(PIXEL_SCALE);
        let x1 = x1.div_euclid(PIXEL_SCALE);
        let y1 = y1.div_euclid(PIXEL_SCALE);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.put_raw(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = error * 2;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn line_raw(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: (u8, u8, u8)) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.put_raw(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = error * 2;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn put_raw(&mut self, x: i32, y: i32, color: (u8, u8, u8)) {
        if x >= 0 && y >= 0 && x < PIXEL_WIDTH as i32 && y < PIXEL_HEIGHT as i32 {
            self.pixels[(y as u32 * PIXEL_WIDTH + x as u32) as usize] =
                pack_rgb(color.0, color.1, color.2);
        }
    }

    fn text_pixel_scale(scale: i32) -> i32 {
        ((scale + 1) / PIXEL_SCALE).max(1)
    }

    fn text_width(&self, value: &str, scale: i32) -> i32 {
        value.chars().count() as i32 * 6 * Self::text_pixel_scale(scale) * PIXEL_SCALE
    }

    fn fit_text(&self, value: &str, max_width: i32, scale: i32) -> String {
        let advance = 6 * Self::text_pixel_scale(scale) * PIXEL_SCALE;
        let max_chars = (max_width / advance).max(1) as usize;
        let length = value.chars().count();
        if length <= max_chars {
            return value.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }
        let mut fitted: String = value.chars().take(max_chars - 3).collect();
        fitted.push_str("...");
        fitted
    }

    fn text_right(&mut self, right: i32, y: i32, value: &str, color: (u8, u8, u8), scale: i32) {
        let x = right - self.text_width(value, scale);
        self.text(x, y, value, color, scale);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_centered(
        &mut self,
        x: i32,
        width: i32,
        y: i32,
        value: &str,
        color: (u8, u8, u8),
        scale: i32,
    ) {
        let value = self.fit_text(value, width, scale);
        let text_width = self.text_width(&value, scale);
        self.text(x + (width - text_width).max(0) / 2, y, &value, color, scale);
    }

    fn text(&mut self, x: i32, y: i32, value: &str, color: (u8, u8, u8), scale: i32) {
        draw_text_line(
            &mut self.pixels,
            PIXEL_WIDTH,
            PIXEL_HEIGHT,
            x.div_euclid(PIXEL_SCALE),
            y.div_euclid(PIXEL_SCALE),
            value,
            pack_rgb(color.0, color.1, color.2),
            Self::text_pixel_scale(scale),
        );
    }

    fn text_wrapped(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        value: &str,
        color: (u8, u8, u8),
        scale: i32,
    ) {
        draw_text_wrapped(
            &mut self.pixels,
            PIXEL_WIDTH,
            PIXEL_HEIGHT,
            x.div_euclid(PIXEL_SCALE),
            y.div_euclid(PIXEL_SCALE),
            width.div_euclid(PIXEL_SCALE),
            value,
            pack_rgb(color.0, color.1, color.2),
            Self::text_pixel_scale(scale),
        );
    }

    fn world(&self, pos: Vec2, ox: f32, oy: f32) -> (i32, i32) {
        ((pos.x * SCALE + ox) as i32, (pos.y * SCALE + oy) as i32)
    }

    fn world_raw(&self, pos: Vec2, ox: f32, oy: f32) -> (i32, i32) {
        let point = self.world(pos, ox, oy);
        (
            point.0.div_euclid(PIXEL_SCALE),
            point.1.div_euclid(PIXEL_SCALE),
        )
    }

    fn shadow_raw(&mut self, cx: i32, cy: i32, width: i32) {
        for y in -3_i32..=3 {
            let row_width = width - y.abs() * 2;
            for x in -row_width..=row_width {
                if (x + y) & 1 == 0 {
                    self.put_raw(cx + x, cy + y, OUTLINE);
                }
            }
        }
    }

    fn blit_sprite(
        &mut self,
        cx: i32,
        cy: i32,
        rows: &[&str],
        main: (u8, u8, u8),
        light: (u8, u8, u8),
        accent: (u8, u8, u8),
        flip_x: bool,
    ) {
        let height = rows.len() as i32;
        let max_width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0) as i32;
        let left = cx - max_width / 2;
        let top = cy - height / 2;
        for (row_index, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            for column in 0..max_width as usize {
                let source = if flip_x {
                    max_width as usize - 1 - column
                } else {
                    column
                };
                let symbol = chars.get(source).copied().unwrap_or('.');
                let color = match symbol {
                    'o' => Some(OUTLINE),
                    'm' => Some(main),
                    'l' => Some(light),
                    'a' => Some(accent),
                    'w' => Some(WHITE),
                    'r' => Some(RED),
                    'c' => Some(CYAN),
                    'v' => Some(VIOLET),
                    'g' => Some(GOLD),
                    'd' => Some(DEEP_VIOLET),
                    's' => Some(STEEL),
                    _ => None,
                };
                if let Some(color) = color {
                    self.put_raw(left + column as i32, top + row_index as i32, color);
                }
            }
        }
    }

    fn paint_bot_player(&mut self, frame: &FrameView, cx: i32, cy: i32) -> bool {
        let Some(bot) = self.art.bot_wheel.as_ref() else {
            return false;
        };
        let player = &frame.player;
        let moving = player.velocity.x.abs() + player.velocity.y.abs() > 8.0;
        let (sheet, animation_frame, anchor_x) = if frame.phase == "dead" {
            let elapsed = (1.35 - frame.death_timer).max(0.0);
            (&bot.death, (elapsed * 7.0).floor().min(5.0) as u32, 25)
        } else if player.hit_flash > 0.0 {
            (&bot.damaged, ((self.time * 14.0) as u32) % 2, 25)
        } else if self.player_dash > 0.0 {
            let elapsed = 0.24 - self.player_dash;
            (
                &bot.dash,
                (elapsed / 0.24 * 7.0).floor().min(6.0) as u32,
                101,
            )
        } else if self.player_shoot > 0.0 {
            let elapsed = 0.28 - self.player_shoot;
            (
                &bot.shoot,
                (elapsed / 0.28 * 4.0).floor().min(3.0) as u32,
                25,
            )
        } else if self.player_respawn > 0.0 {
            let elapsed = 0.52 - self.player_respawn;
            (
                &bot.wake,
                (elapsed / 0.52 * 5.0).floor().min(4.0) as u32,
                25,
            )
        } else if player.reload_timer > 0.0 {
            (&bot.charge, ((self.time * 9.0) as u32) % 4, 25)
        } else if moving {
            (&bot.movement, ((self.time * 10.0) as u32) % 8, 25)
        } else {
            (&bot.idle, 0, 25)
        };
        let flip = player.aim.x < 0.0;
        let anchor_x = if flip {
            sheet.width.saturating_sub(1 + anchor_x)
        } else {
            anchor_x
        };
        let tint = if player.hit_flash > 0.0 {
            Some((WHITE, 0.8))
        } else if player.invulnerable > 0.0 && ((self.time * 12.0) as i32) & 1 == 1 {
            Some((VIOLET, 0.62))
        } else {
            Some((CYAN, 0.5))
        };
        blit_rgba_region(
            &mut self.pixels,
            sheet,
            0,
            animation_frame * 26,
            sheet.width,
            26,
            cx - anchor_x as i32,
            cy - 17,
            flip,
            tint,
            1.0,
        );
        true
    }

    fn paint_bot_portrait(
        &mut self,
        cx: i32,
        cy: i32,
        animation_frame: u32,
        waking: bool,
        pixel_size: i32,
    ) -> bool {
        let Some(bot) = self.art.bot_wheel.as_ref() else {
            return false;
        };
        let (sheet, frames) = if waking {
            (&bot.wake, 5)
        } else {
            (&bot.idle, 1)
        };
        blit_rgba_region_scaled(
            &mut self.pixels,
            sheet,
            0,
            animation_frame.min(frames - 1) * 26,
            52,
            26,
            cx - 25 * pixel_size,
            cy - 17 * pixel_size,
            pixel_size,
            false,
            Some((CYAN, 0.54)),
            1.0,
        );
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_neo_character(
        &mut self,
        cx: i32,
        cy: i32,
        base_row: u32,
        aim: Vec2,
        moving: bool,
        tint: Option<((u8, u8, u8), f32)>,
        opacity: f32,
    ) -> bool {
        let Some(sheet) = self.art.characters.as_ref() else {
            return false;
        };
        let direction = character_direction_row(aim);
        let frame = if moving {
            ((self.time * 8.0) as u32) % 3
        } else {
            1
        };
        blit_rgba_region(
            &mut self.pixels,
            sheet,
            frame * 32,
            (base_row + direction) * 24,
            32,
            24,
            cx - 16,
            cy - 19,
            false,
            tint,
            opacity,
        );
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn blit_sprite_scaled(
        &mut self,
        cx: i32,
        cy: i32,
        rows: &[&str],
        main: (u8, u8, u8),
        light: (u8, u8, u8),
        accent: (u8, u8, u8),
        flip_x: bool,
        pixel_size: i32,
    ) {
        let height = rows.len() as i32;
        let max_width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0) as i32;
        let left = cx - max_width * pixel_size / 2;
        let top = cy - height * pixel_size / 2;
        for (row_index, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            for column in 0..max_width as usize {
                let source = if flip_x {
                    max_width as usize - 1 - column
                } else {
                    column
                };
                let symbol = chars.get(source).copied().unwrap_or('.');
                let color = match symbol {
                    'o' => Some(OUTLINE),
                    'm' => Some(main),
                    'l' => Some(light),
                    'a' => Some(accent),
                    'w' => Some(WHITE),
                    'r' => Some(RED),
                    'c' => Some(CYAN),
                    'v' => Some(VIOLET),
                    'g' => Some(GOLD),
                    'd' => Some(DEEP_VIOLET),
                    's' => Some(STEEL),
                    _ => None,
                };
                if let Some(color) = color {
                    self.rect_raw(
                        left + column as i32 * pixel_size,
                        top + row_index as i32 * pixel_size,
                        pixel_size,
                        pixel_size,
                        color,
                    );
                }
            }
        }
    }

    fn world_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: (u8, u8, u8),
        ox: f32,
        oy: f32,
    ) {
        self.rect(
            (x * SCALE + ox) as i32,
            (y * SCALE + oy) as i32,
            (w * SCALE).ceil() as i32,
            (h * SCALE).ceil() as i32,
            color,
        );
    }

    fn world_line(&mut self, from: Vec2, to: Vec2, color: (u8, u8, u8), ox: f32, oy: f32) {
        let a = self.world(from, ox, oy);
        let b = self.world(to, ox, oy);
        self.line(a.0, a.1, b.0, b.1, color);
    }

    fn tint(&mut self, color: (u8, u8, u8), alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        for pixel in &mut self.pixels {
            let r = ((*pixel >> 16) & 255) as f32;
            let g = ((*pixel >> 8) & 255) as f32;
            let b = (*pixel & 255) as f32;
            *pixel = pack_rgb(
                (r * (1.0 - alpha) + color.0 as f32 * alpha) as u8,
                (g * (1.0 - alpha) + color.1 as f32 * alpha) as u8,
                (b * (1.0 - alpha) + color.2 as f32 * alpha) as u8,
            );
        }
    }

    fn scanlines(&mut self, strength: f32) {
        for y in (1..PIXEL_HEIGHT).step_by(2) {
            for x in 0..PIXEL_WIDTH {
                let index = (y * PIXEL_WIDTH + x) as usize;
                let pixel = self.pixels[index];
                let factor = 1.0 - strength;
                self.pixels[index] = pack_rgb(
                    (((pixel >> 16) & 255) as f32 * factor) as u8,
                    (((pixel >> 8) & 255) as f32 * factor) as u8,
                    ((pixel & 255) as f32 * factor) as u8,
                );
            }
        }
    }

    fn glitch(&mut self, strength: f32) {
        let strength = strength.clamp(0.0, 1.0);
        let source = self.pixels.clone();
        let bands = (strength * 18.0) as i32 + 1;
        for band in 0..bands {
            let y = ((self.time * 53.0) as i32 + band * 23).rem_euclid(PIXEL_HEIGHT as i32);
            let height = 1 + (band * 5).rem_euclid(5);
            let offset = (((band * 13 + self.time as i32 * 11) % 17) - 8) as f32 * strength;
            for py in y..(y + height).min(PIXEL_HEIGHT as i32) {
                for x in 0..PIXEL_WIDTH as i32 {
                    let sx = (x as f32 + offset).clamp(0.0, PIXEL_WIDTH as f32 - 1.0) as u32;
                    self.pixels[(py as u32 * PIXEL_WIDTH + x as u32) as usize] =
                        source[(py as u32 * PIXEL_WIDTH + sx) as usize];
                }
            }
        }
    }
}

fn character_direction_row(aim: Vec2) -> u32 {
    if aim.x.abs() > aim.y.abs() {
        if aim.x < 0.0 {
            3
        } else {
            2
        }
    } else if aim.y < 0.0 {
        1
    } else {
        0
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_rgba_region(
    destination: &mut [u32],
    sheet: &SpriteSheet,
    source_x: u32,
    source_y: u32,
    width: u32,
    height: u32,
    destination_x: i32,
    destination_y: i32,
    flip_x: bool,
    tint: Option<((u8, u8, u8), f32)>,
    opacity: f32,
) {
    blit_rgba_region_scaled(
        destination,
        sheet,
        source_x,
        source_y,
        width,
        height,
        destination_x,
        destination_y,
        1,
        flip_x,
        tint,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
fn blit_rgba_region_scaled(
    destination: &mut [u32],
    sheet: &SpriteSheet,
    source_x: u32,
    source_y: u32,
    width: u32,
    height: u32,
    destination_x: i32,
    destination_y: i32,
    pixel_size: i32,
    flip_x: bool,
    tint: Option<((u8, u8, u8), f32)>,
    opacity: f32,
) {
    if pixel_size <= 0 {
        return;
    }
    if source_x + width > sheet.width || source_y + height > sheet.height {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let sx = source_x + if flip_x { width - 1 - x } else { x };
            let source_index = ((source_y + y) * sheet.width + sx) as usize * 4;
            let source_alpha = sheet.rgba[source_index + 3] as f32 / 255.0 * opacity;
            if source_alpha <= 0.0 {
                continue;
            }
            let mut source = (
                sheet.rgba[source_index],
                sheet.rgba[source_index + 1],
                sheet.rgba[source_index + 2],
            );
            if let Some((color, strength)) = tint {
                source = mix(source, color, strength);
            }
            for py in 0..pixel_size {
                let dy = destination_y + y as i32 * pixel_size + py;
                if !(0..PIXEL_HEIGHT as i32).contains(&dy) {
                    continue;
                }
                for px in 0..pixel_size {
                    let dx = destination_x + x as i32 * pixel_size + px;
                    if !(0..PIXEL_WIDTH as i32).contains(&dx) {
                        continue;
                    }
                    let destination_index = (dy as u32 * PIXEL_WIDTH + dx as u32) as usize;
                    if source_alpha >= 0.995 {
                        destination[destination_index] = pack_rgb(source.0, source.1, source.2);
                        continue;
                    }
                    let background = destination[destination_index];
                    let background = (
                        ((background >> 16) & 255) as u8,
                        ((background >> 8) & 255) as u8,
                        (background & 255) as u8,
                    );
                    let blended = mix(background, source, source_alpha);
                    destination[destination_index] = pack_rgb(blended.0, blended.1, blended.2);
                }
            }
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "SI"
    } else {
        "NO"
    }
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 * (1.0 - t) + b.0 as f32 * t) as u8,
        (a.1 as f32 * (1.0 - t) + b.1 as f32 * t) as u8,
        (a.2 as f32 * (1.0 - t) + b.2 as f32 * t) as u8,
    )
}

pub fn scale_nearest_letterbox(src: &[u32], dw: u32, dh: u32) -> Vec<u32> {
    let mut output = vec![pack_rgb(0, 0, 0); (dw * dh) as usize];
    let scale = (dw as f32 / PIXEL_WIDTH as f32).min(dh as f32 / PIXEL_HEIGHT as f32);
    let target_w = (PIXEL_WIDTH as f32 * scale).round().max(1.0) as u32;
    let target_h = (PIXEL_HEIGHT as f32 * scale).round().max(1.0) as u32;
    let offset_x = (dw - target_w) / 2;
    let offset_y = (dh - target_h) / 2;
    for y in 0..target_h {
        let sy = y * PIXEL_HEIGHT / target_h;
        for x in 0..target_w {
            let sx = x * PIXEL_WIDTH / target_w;
            output[((y + offset_y) * dw + x + offset_x) as usize] =
                src[(sy * PIXEL_WIDTH + sx) as usize];
        }
    }
    output
}
