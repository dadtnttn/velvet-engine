//! The gallery in the title's manner: the band down the left with the albums (the prologue's
//! pictures in one, then the animated scenes by place: the castle, the brothel, the dark forest,
//! the shop), a grid of slanted cards on the right (what has not been seen yet, locked), and a
//! viewer: a picture whole with its line, or a scene's shots playing in a loop with their sound,
//! one card per shot. The pictures unlock as the visual novel shows them; a scene's shots, as
//! they play in the game (`exploration3d::gallery_seen`). Thumbnails are made at their size by
//! `tools/gallery_thumbs.py` and drawn 1:1. Focus only recolours; nothing moves under the pointer.

use std::path::PathBuf;
use std::time::Instant;

use winit::keyboard::KeyCode;

use crate::exploration3d::gallery_seen;
use crate::exploration3d::ui_sfx::{self, UiSfx};
use crate::localization::Language;
use crate::menu_ui::{self as ui, Rect, GOLD, INK, MUTED};
use crate::prompt_icons::Icon;
use crate::title_reel::TitleReel;
use crate::{title_menu, App};

struct SceneDef {
    es: &'static str,
    en: &'static str,
    /// Its shots' folder under `data/assets` (one folder of frames per shot).
    dir: &'static str,
}

struct AlbumDef {
    es: &'static str,
    en: &'static str,
    icon: Icon,
    /// Empty: the prologue's pictures.
    scenes: &'static [SceneDef],
}

const ALBUMS: [AlbumDef; 5] = [
    AlbumDef {
        es: "Prólogo",
        en: "Prologue",
        icon: Icon::Memory,
        scenes: &[],
    },
    AlbumDef {
        es: "Castillo",
        en: "Castle",
        icon: Icon::Castle,
        scenes: &[SceneDef {
            es: "La alcoba",
            en: "The alcove",
            dir: "castillo/escena_cama",
        }],
    },
    AlbumDef {
        es: "Burdel",
        en: "Brothel",
        icon: Icon::Heart,
        scenes: &[
            SceneDef {
                es: "Nina",
                en: "Nina",
                dir: "burdel/nina/escena",
            },
            SceneDef {
                es: "Nina · elixir",
                en: "Nina · elixir",
                dir: "burdel/nina/escena_futa",
            },
            SceneDef {
                es: "Lo que pide Alba",
                en: "What Alba asks for",
                dir: "burdel/alba/escena",
            },
            SceneDef {
                es: "Alba · elixir",
                en: "Alba · elixir",
                dir: "burdel/alba/escena_futa",
            },
            SceneDef {
                es: "Silencio",
                en: "Silence",
                dir: "burdel/noa/escena",
            },
            SceneDef {
                es: "Noa · elixir",
                en: "Noa · elixir",
                dir: "burdel/noa/escena_futa",
            },
        ],
    },
    AlbumDef {
        es: "Bosque Oscuro",
        en: "Dark Forest",
        icon: Icon::Forest,
        scenes: &[SceneDef {
            es: "No mires el anillo",
            en: "Don't look at the ring",
            dir: "hipno/escena",
        }],
    },
    AlbumDef {
        es: "Tienda",
        en: "Shop",
        icon: Icon::Shop,
        scenes: &[SceneDef {
            es: "Otro trato",
            en: "Another deal",
            dir: "tienda/escena",
        }],
    },
];

/// The list down the band: the albums, then «Volver».
const LIST: usize = ALBUMS.len() + 1;
const COLS: usize = 4;
const PER_PAGE: usize = 12;
const TILE_W: i32 = 272;
const TILE_H: i32 = 160;
const GRID_X: i32 = 690;
const GRID_Y: i32 = 236;
const GAP_X: i32 = 24;
const ROW_PITCH: i32 = 214;
const PAGE_PREVIOUS: Rect = (1676, 150, 76, 50);
const PAGE_NEXT: Rect = (1772, 150, 76, 50);
/// A scene's shot cards in the viewer: five to a row.
const SHOT_COLS: usize = 5;

/// A scene's shots: folder name, folder, seen.
pub(crate) struct Shots {
    dir: PathBuf,
    shots: Vec<(String, bool)>,
}

impl Shots {
    fn seen(&self) -> usize {
        self.shots.iter().filter(|(_, seen)| *seen).count()
    }
}

pub(crate) enum Viewer {
    /// The `n`th picture of the prologue.
    Still(usize),
    /// A scene (album, scene) and the shot playing.
    Scene {
        album: usize,
        scene: usize,
        shot: usize,
        reel: Option<TitleReel>,
    },
}

pub(crate) struct GalleryUi {
    pub album: usize,
    /// The keyboard is on the grid (else on the list down the band).
    pub on_grid: bool,
    pub list: usize,
    pub tile: usize,
    pub since: Instant,
    /// Each scene album's scenes (the prologue's entry is empty).
    scenes: Vec<Vec<Shots>>,
    pub viewer: Option<Viewer>,
    /// The still layer under the grid (the place, its shade, the band) once the entrance is
    /// over, and the place it was painted for: copied instead of painted again.
    backdrop: Vec<u32>,
    backdrop_place: Option<usize>,
}

impl Default for GalleryUi {
    fn default() -> Self {
        Self {
            album: 0,
            on_grid: false,
            list: 0,
            tile: 0,
            since: Instant::now(),
            scenes: Vec::new(),
            viewer: None,
            backdrop: Vec::new(),
            backdrop_place: None,
        }
    }
}

/// Where a shot comes in its scene: the way in first, the end and the calm last.
fn shot_rank(name: &str) -> (u8, String) {
    let rank = if name.starts_with("intro") || name == "l0" || name == "p0" {
        0
    } else if name.starts_with("climax") || name.starts_with("c") && name.len() <= 3 {
        8
    } else if name.starts_with("relax") || name.starts_with("final") {
        9
    } else {
        4
    };
    (rank, name.to_owned())
}

fn t(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language != Language::English {
        es
    } else {
        en
    }
}

fn list_rect(index: usize) -> Rect {
    (70 + index as i32 * 12, 330 + index as i32 * 84, 410, 68)
}

fn tile_rect(slot: usize) -> Rect {
    let (col, row) = ((slot % COLS) as i32, (slot / COLS) as i32);
    (
        GRID_X + col * (TILE_W + GAP_X),
        GRID_Y + row * ROW_PITCH,
        TILE_W,
        TILE_H,
    )
}

fn shot_rect(index: usize) -> Rect {
    let (col, row) = ((index % SHOT_COLS) as i32, (index / SHOT_COLS) as i32);
    (44 + col * 78, 360 + row * 58, 70, 46)
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
}

/// A picture's thumbnail (see `tools/gallery_thumbs.py`).
fn thumb_key(asset: &str) -> String {
    format!("gallery/thumbs/{}.jpg", asset.replace('/', "__"))
}

fn scene_thumb_key(dir: &str) -> String {
    format!("gallery/thumbs/escena__{}.jpg", dir.replace('/', "__"))
}

impl GalleryUi {
    /// What has been seen, read again (on opening the gallery).
    pub(crate) fn reload(&mut self) {
        let seen = crate::game::gallery_unlocked();
        let root = crate::assets_root();
        self.scenes = ALBUMS
            .iter()
            .map(|album| {
                album
                    .scenes
                    .iter()
                    .map(|scene| {
                        let dir = root.join(scene.dir);
                        let mut names: Vec<String> = std::fs::read_dir(&dir)
                            .map(|d| {
                                d.flatten()
                                    .filter(|e| crate::exploration3d::clip_video::is_shot(&e.path()))
                                    .map(|e| e.file_name().to_string_lossy().into_owned())
                                    .collect()
                            })
                            .unwrap_or_default();
                        names.sort_by_key(|n| shot_rank(n));
                        let shots = names
                            .into_iter()
                            .map(|n| {
                                let key = format!("{}{}/{n}", gallery_seen::PREFIX, scene.dir);
                                let seen = seen.contains(&key);
                                (n, seen)
                            })
                            .collect();
                        Shots { dir, shots }
                    })
                    .collect()
            })
            .collect();
        self.album = self.album.min(ALBUMS.len() - 1);
        self.list = self.list.min(LIST - 1);
        self.viewer = None;
        self.since = Instant::now();
        self.backdrop_place = None;
    }

    fn shots(&self, album: usize, scene: usize) -> Option<&Shots> {
        self.scenes.get(album)?.get(scene)
    }

    /// Is a scene's card open (any of its shots seen)?
    fn scene_open(&self, album: usize, scene: usize) -> bool {
        self.shots(album, scene).is_some_and(|s| s.seen() > 0)
    }

    /// Does the gallery move by itself (a scene playing)?
    pub(crate) fn playing(&self) -> bool {
        matches!(self.viewer, Some(Viewer::Scene { .. }))
    }
}

impl App {
    /// The cards of album `album`: how many, and how many of them open.
    fn gallery_counts(&self, album: usize) -> (usize, usize) {
        if album == 0 {
            (self.gallery.total.max(self.gallery.entries.len()), self.gallery.entries.len())
        } else {
            let scenes = ALBUMS[album].scenes.len();
            let open = (0..scenes)
                .filter(|&s| self.gallery_ui.scene_open(album, s))
                .count();
            (scenes, open)
        }
    }

    /// Seen and in all, counting each scene's shots.
    fn gallery_totals(&self) -> (usize, usize) {
        let mut seen = self.gallery.entries.len();
        let mut all = self.gallery.total.max(seen);
        for album in &self.gallery_ui.scenes {
            for scene in album {
                seen += scene.seen();
                all += scene.shots.len();
            }
        }
        (seen, all)
    }

    fn gallery_card_open(&self, album: usize, index: usize) -> bool {
        if album == 0 {
            index < self.gallery.entries.len()
        } else {
            self.gallery_ui.scene_open(album, index)
        }
    }

    /// Opens card `index` of the album on show (nothing for a locked one).
    fn gallery_open_card(&mut self, index: usize) {
        let album = self.gallery_ui.album;
        if !self.gallery_card_open(album, index) {
            ui_sfx::play(UiSfx::Back);
            return;
        }
        ui_sfx::play(UiSfx::Select);
        if album == 0 {
            self.gallery_ui.viewer = Some(Viewer::Still(index));
        } else {
            let Some(shots) = self.gallery_ui.shots(album, index) else {
                return;
            };
            let first = shots.shots.iter().position(|(_, seen)| *seen).unwrap_or(0);
            let reel = TitleReel::open(shots.dir.clone());
            self.gallery_ui.viewer = Some(Viewer::Scene {
                album,
                scene: index,
                shot: usize::MAX,
                reel,
            });
            self.gallery_play_shot(first);
        }
        self.needs_paint = true;
    }

    /// Shot `index` of the scene on show, with its sound.
    fn gallery_play_shot(&mut self, index: usize) {
        let (album, scene) = match &self.gallery_ui.viewer {
            Some(Viewer::Scene { album, scene, .. }) => (*album, *scene),
            _ => return,
        };
        let Some(shots) = self.gallery_ui.shots(album, scene) else {
            return;
        };
        let Some((name, seen)) = shots.shots.get(index).cloned() else {
            return;
        };
        if !seen {
            ui_sfx::play(UiSfx::Back);
            return;
        }
        let audio = shots.dir.join(&name).join("audio.ogg");
        if let Some(Viewer::Scene { shot, reel, .. }) = &mut self.gallery_ui.viewer {
            if *shot == index {
                return;
            }
            *shot = index;
            if let Some(reel) = reel.as_mut() {
                reel.play(&name);
            }
        }
        ui_sfx::scene_loop(audio.is_file().then_some(audio.as_path()));
        self.needs_paint = true;
    }

    /// The viewer's next (or previous) picture or seen shot.
    fn gallery_viewer_step(&mut self, by: i32) {
        match &self.gallery_ui.viewer {
            Some(Viewer::Still(n)) => {
                let count = self.gallery.entries.len().max(1) as i32;
                let next = (*n as i32 + by).rem_euclid(count) as usize;
                self.gallery_ui.viewer = Some(Viewer::Still(next));
                ui_sfx::play(UiSfx::Move);
            }
            Some(Viewer::Scene {
                album, scene, shot, ..
            }) => {
                let (album, scene, shot) = (*album, *scene, *shot);
                let Some(shots) = self.gallery_ui.shots(album, scene) else {
                    return;
                };
                let n = shots.shots.len() as i32;
                let mut at = shot as i32;
                for _ in 0..n {
                    at = (at + by).rem_euclid(n);
                    if shots.shots[at as usize].1 {
                        break;
                    }
                }
                ui_sfx::play(UiSfx::Move);
                self.gallery_play_shot(at as usize);
            }
            None => {}
        }
        self.needs_paint = true;
    }

    /// Closes the viewer (true) or there was none (false).
    pub(crate) fn gallery_close_viewer(&mut self) -> bool {
        if self.gallery_ui.viewer.take().is_none() {
            return false;
        }
        ui_sfx::scene_loop(None);
        self.ui_hidden = false;
        self.needs_paint = true;
        true
    }

    fn gallery_select_album(&mut self, album: usize) {
        if self.gallery_ui.album != album {
            self.gallery_ui.album = album;
            self.gallery_ui.tile = 0;
            self.gallery_ui.since = Instant::now();
        }
    }

    pub(crate) fn gallery_key(&mut self, key: KeyCode) {
        if self.gallery_ui.viewer.is_some() {
            match key {
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.gallery_viewer_step(-1)
                }
                KeyCode::ArrowRight
                | KeyCode::KeyD
                | KeyCode::ArrowDown
                | KeyCode::KeyS
                | KeyCode::Space
                | KeyCode::Enter
                | KeyCode::NumpadEnter => self.gallery_viewer_step(1),
                KeyCode::Escape | KeyCode::Backspace => {
                    if self.gallery_close_viewer() {
                        ui_sfx::play(UiSfx::Back);
                    }
                }
                _ => {}
            }
            return;
        }
        if key == KeyCode::Escape || key == KeyCode::Backspace {
            self.leave_gallery();
            return;
        }
        let album = self.gallery_ui.album;
        let (count, _) = self.gallery_counts(album);
        let ui = &mut self.gallery_ui;
        let before = (ui.on_grid, ui.list, ui.tile, ui.album);
        if !ui.on_grid {
            match key {
                KeyCode::ArrowUp | KeyCode::KeyW => ui.list = (ui.list + LIST - 1) % LIST,
                KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => ui.list = (ui.list + 1) % LIST,
                KeyCode::ArrowRight | KeyCode::KeyD if ui.list < ALBUMS.len() && count > 0 => {
                    ui.on_grid = true
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    if ui.list == ALBUMS.len() {
                        self.leave_gallery();
                        return;
                    }
                    if count > 0 {
                        ui.on_grid = true;
                    }
                }
                _ => {}
            }
            let list = self.gallery_ui.list;
            if list < ALBUMS.len() {
                self.gallery_select_album(list);
            }
        } else {
            let col = ui.tile % COLS;
            match key {
                KeyCode::ArrowLeft | KeyCode::KeyA if col == 0 => ui.on_grid = false,
                KeyCode::ArrowLeft | KeyCode::KeyA => ui.tile -= 1,
                KeyCode::ArrowRight | KeyCode::KeyD => ui.tile = (ui.tile + 1).min(count - 1),
                KeyCode::ArrowUp | KeyCode::KeyW if ui.tile >= COLS => ui.tile -= COLS,
                KeyCode::ArrowDown | KeyCode::KeyS if ui.tile + COLS < count => ui.tile += COLS,
                KeyCode::KeyQ | KeyCode::PageUp => ui.tile = ui.tile.saturating_sub(PER_PAGE),
                KeyCode::KeyE | KeyCode::PageDown => ui.tile = (ui.tile + PER_PAGE).min(count - 1),
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    let tile = ui.tile;
                    self.gallery_open_card(tile);
                    return;
                }
                _ => {}
            }
        }
        let ui = &self.gallery_ui;
        if before != (ui.on_grid, ui.list, ui.tile, ui.album) {
            ui_sfx::play(UiSfx::Move);
        }
        self.needs_paint = true;
    }

    pub(crate) fn gallery_click(&mut self, x: i32, y: i32) {
        match &self.gallery_ui.viewer {
            Some(Viewer::Still(_)) => {
                self.gallery_viewer_step(if x < 960 { -1 } else { 1 });
                return;
            }
            Some(Viewer::Scene { album, scene, .. }) => {
                let n = self
                    .gallery_ui
                    .shots(*album, *scene)
                    .map_or(0, |s| s.shots.len());
                if let Some(i) = (0..n).find(|&i| inside(shot_rect(i), x, y)) {
                    self.gallery_play_shot(i);
                }
                return;
            }
            None => {}
        }
        if let Some(i) = (0..LIST).find(|&i| inside(list_rect(i), x, y)) {
            self.gallery_ui.list = i;
            self.gallery_ui.on_grid = false;
            if i == ALBUMS.len() {
                self.leave_gallery();
            } else {
                ui_sfx::play(UiSfx::Select);
                self.gallery_select_album(i);
            }
            self.needs_paint = true;
            return;
        }
        let album = self.gallery_ui.album;
        let (count, _) = self.gallery_counts(album);
        let page = self.gallery_ui.tile / PER_PAGE;
        let pages = count.div_ceil(PER_PAGE).max(1);
        if inside(PAGE_PREVIOUS, x, y) && page > 0 {
            self.gallery_ui.tile = (page - 1) * PER_PAGE;
            ui_sfx::play(UiSfx::Move);
        } else if inside(PAGE_NEXT, x, y) && page + 1 < pages {
            self.gallery_ui.tile = (page + 1) * PER_PAGE;
            ui_sfx::play(UiSfx::Move);
        } else if let Some(slot) = (0..PER_PAGE).find(|&s| inside(tile_rect(s), x, y)) {
            let index = page * PER_PAGE + slot;
            if index < count {
                self.gallery_ui.tile = index;
                self.gallery_ui.on_grid = true;
                self.gallery_open_card(index);
            }
        }
        self.needs_paint = true;
    }

    /// For captures: card `card` of the album on show open, a scene's shots all shown as seen and
    /// `shot` playing (its first frame decoded before this returns).
    pub(crate) fn gallery_capture_card(&mut self, card: usize, shot: Option<usize>) {
        let album = self.gallery_ui.album;
        if album > 0 {
            if let Some(shots) = self.gallery_ui.scenes.get_mut(album).and_then(|a| a.get_mut(card)) {
                for (_, seen) in &mut shots.shots {
                    *seen = true;
                }
            }
        }
        self.gallery_ui.tile = card;
        self.gallery_open_card(card);
        if let Some(shot) = shot {
            self.gallery_play_shot(shot);
        }
        if let Some(Viewer::Scene { reel: Some(reel), .. }) = &mut self.gallery_ui.viewer {
            for _ in 0..200 {
                if reel.frame().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    pub(crate) fn gallery_scroll(&mut self, up: bool) {
        if self.gallery_ui.viewer.is_some() {
            self.gallery_viewer_step(if up { -1 } else { 1 });
        } else {
            self.gallery_key(if up { KeyCode::PageUp } else { KeyCode::PageDown });
        }
    }

    pub(crate) fn paint_gallery_persona(&mut self) {
        match &self.gallery_ui.viewer {
            Some(Viewer::Still(n)) => {
                let n = *n;
                self.paint_gallery_still(n);
                return;
            }
            Some(Viewer::Scene { .. }) => {
                self.paint_gallery_scene();
                return;
            }
            None => {}
        }
        let motion = !self.reduced_motion;
        let age = self.gallery_ui.since.elapsed().as_secs_f32();
        let clock = self.title_since.elapsed().as_secs_f32();
        let place = self.title_background_current.min(title_menu::PLACES.len() - 1);
        let cached = self.gallery_ui.backdrop_place == Some(place)
            && self.gallery_ui.backdrop.len() == self.canvas.pixels.len()
            && !self.ui_hidden;
        if cached {
            self.canvas.pixels.copy_from_slice(&self.gallery_ui.backdrop);
        } else {
            self.canvas.clear(0xff0b0710);
            if let Some(b) = self.art.original(title_menu::PLACES[place].0) {
                self.canvas.blit_rgba(0, 0, b);
            }
        }
        if self.ui_hidden {
            return;
        }
        let light = self.title_light(place);
        let accent = ui::accent(light);
        let language = self.localizer.language();
        let t = |es, en| t(language, es, en);
        let album = self.gallery_ui.album;
        let (count, open) = self.gallery_counts(album);
        let (seen, all) = self.gallery_totals();
        let counts: Vec<(usize, usize)> = (0..ALBUMS.len()).map(|a| self.gallery_counts(a)).collect();
        let ui_state = (
            self.gallery_ui.on_grid,
            self.gallery_ui.list,
            self.gallery_ui.tile,
        );
        let mouse = self.mouse;
        if !cached {
            self.canvas.rect(0, 0, 1920, 1080, 0x9a08040e);
            title_menu::paint_band(&mut self.canvas, accent, 0.0, clock, motion);
            // Once the cards are in, the layer under them stays as it is.
            if !motion || age > 1.2 {
                self.gallery_ui.backdrop.clone_from(&self.canvas.pixels);
                self.gallery_ui.backdrop_place = Some(place);
            }
        }
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let canvas = &mut self.canvas;

        // ── The band: the name, how much has been seen, the albums and the way back.
        let title = t("Galería", "Gallery");
        canvas.display_text(fonts, title, 72 + 5, 74 + 5, 80.0, 0xff000000);
        canvas.display_text(fonts, title, 72, 74, 80.0, 0xffffffff);
        ui::tag(
            canvas,
            fonts,
            80,
            182,
            t("Recuerdos de la Corona", "Memories of the Crown"),
            15.0,
            ui::ABYSS,
            ui::BONE,
        );
        canvas.line((80.0, 246.0), (470.0, 236.0), 2.0, 0xccffffff);
        ui::text_mid(
            canvas,
            fonts,
            &format!("{seen} / {all}"),
            82,
            282,
            22.0,
            0xffffffff,
            true,
        );
        let k = all.max(1) as f32;
        ui::meter(canvas, 190, 270, 250, 22, seen as f32 / k, GOLD & 0x00ff_ffff);
        for i in 0..LIST {
            let rect = list_rect(i);
            let over = inside(rect, mouse.0, mouse.1);
            let keyed = !ui_state.0 && ui_state.1 == i;
            let label = if i < ALBUMS.len() {
                format!(
                    "{} · {}/{}",
                    t(ALBUMS[i].es, ALBUMS[i].en),
                    counts[i].1,
                    counts[i].0
                )
            } else {
                t("Volver", "Back").to_string()
            };
            let icon = if i < ALBUMS.len() { ALBUMS[i].icon } else { Icon::Leave };
            let selected = keyed || over || (i == album && i < ALBUMS.len());
            title_menu::plate(
                canvas,
                fonts,
                rect,
                0.0,
                &label,
                icon,
                true,
                selected,
                false,
                accent,
                clock,
                motion,
                over && self.mouse_down,
            );
            if keyed {
                title_menu::pointer(canvas, (rect.0 as f32 - 12.0, (rect.1 + rect.3 / 2) as f32), 20.0);
            }
        }

        // ── The album: its name, what it holds, its page.
        let name = t(ALBUMS[album].es, ALBUMS[album].en);
        ui::plate(canvas, 700.0, 70.0, 640.0, 74.0, 26.0, 0xb4000000);
        ui::plate(canvas, 690.0, 60.0, 640.0, 74.0, 26.0, ui::argb(0xff, accent));
        let ink = ui::ink_on(accent);
        let size = ui::fit_size(fonts, name, 520, 46.0, false);
        if ink == 0xffff_ffff {
            canvas.display_text(fonts, name, 742 + 3, 66 + 3, size, 0xff000000);
        }
        canvas.display_text(fonts, name, 742, 66, size, ink);
        let what = if album == 0 {
            t(
                "Las imágenes de la novela visual, según las vas viendo.",
                "The visual novel's pictures, as you see them.",
            )
        } else {
            t(
                "Las escenas animadas: cada toma se abre al verla en el juego.",
                "The animated scenes: each shot opens once you see it in the game.",
            )
        };
        ui::text_mid(canvas, fonts, what, 694, 178, 18.0, INK, false);
        let pages = count.div_ceil(PER_PAGE).max(1);
        let page = (ui_state.2 / PER_PAGE).min(pages - 1);
        if pages > 1 {
            ui::pill(
                canvas,
                fonts,
                PAGE_PREVIOUS,
                "‹",
                inside(PAGE_PREVIOUS, mouse.0, mouse.1),
                page > 0,
                light,
                false,
            );
            ui::pill(
                canvas,
                fonts,
                PAGE_NEXT,
                "›",
                inside(PAGE_NEXT, mouse.0, mouse.1),
                page + 1 < pages,
                light,
                false,
            );
            ui::text_right(
                canvas,
                fonts,
                &format!("{} / {pages}", page + 1),
                PAGE_PREVIOUS.0 - 18,
                PAGE_PREVIOUS.1 + PAGE_PREVIOUS.3 / 2,
                20.0,
                INK,
                true,
            );
        }
        let _ = open;

        // ── The cards: thumbnails in slanted frames, the locked ones dark with a lock.
        for slot in 0..PER_PAGE {
            let index = page * PER_PAGE + slot;
            if index >= count {
                break;
            }
            let rect = tile_rect(slot);
            let come = if motion {
                ui::entrance(age, 0.04 * slot as f32, 0.26)
            } else {
                1.0
            };
            if come <= 0.0 {
                continue;
            }
            let (x, y, w, h) = rect;
            let over = inside(rect, mouse.0, mouse.1);
            let focused = (ui_state.0 && ui_state.2 == index) || over;
            let open = if album == 0 {
                index < self.gallery.entries.len()
            } else {
                self.gallery_ui.scene_open(album, index)
            };
            let (fx, fy, fw, fh) = (x as f32 - 8.0, y as f32 - 8.0, w as f32 + 16.0, h as f32 + 16.0);
            ui::plate(canvas, fx + 10.0, fy + 10.0, fw, fh, 16.0, 0xb0000000);
            ui::plate(
                canvas,
                fx,
                fy,
                fw,
                fh,
                16.0,
                if focused {
                    ui::argb(0xff, accent)
                } else {
                    0xff15101c
                },
            );
            canvas.rect(x + 8, y + 8, 256, 144, 0xff0c0811);
            let key = if !open {
                None
            } else if album == 0 {
                Some(thumb_key(&self.gallery.entries[index].asset))
            } else {
                Some(scene_thumb_key(ALBUMS[album].scenes[index].dir))
            };
            let drawn = key
                .as_deref()
                .and_then(|k| self.art.original(k))
                .map(|b| canvas.blit_rgba(x + 8, y + 8, b))
                .is_some();
            if !drawn {
                // Locked (or no thumbnail yet): halftone and a lock.
                ui::halftone(
                    canvas,
                    ((x + 136) as f32, (y + 80) as f32, 128.0, 72.0),
                    12.0,
                    3.0,
                    ((x + 264) as f32, (y + 152) as f32),
                    220.0,
                    ui::argb(0x40, accent),
                );
                ui::icon_disc(canvas, Icon::Lock, x + 136, y + 80, 28, accent, false);
            }
            if come < 1.0 {
                let a = ((1.0 - come) * 255.0) as u32;
                canvas.rect(x - 8, y - 8, w + 16, h + 16, a << 24 | 0x0b0710);
            }
            ui::outline(
                canvas,
                &ui::plate_points(fx, fy, fw, fh, 16.0),
                if focused { 2.4 } else { 1.0 },
                if focused { 0xffffffff } else { 0x40ffffff },
            );
            ui::tag(
                canvas,
                fonts,
                x + 4,
                y + h - 28,
                &format!("{:02}", index + 1),
                14.0,
                if focused { ui::BONE } else { ui::ABYSS },
                if focused { ui::ABYSS } else { ui::BONE },
            );
            if album > 0 {
                let scene = &ALBUMS[album].scenes[index];
                let label = if open { t(scene.es, scene.en) } else { "???" };
                let shots = self.gallery_ui.shots(album, index);
                let (s_seen, s_all) = shots.map_or((0, 0), |s| (s.seen(), s.shots.len()));
                let size = ui::fit_size(fonts, label, w - 70, 20.0, true);
                ui::text_mid(canvas, fonts, label, x, y + h + 26, size, INK, true);
                ui::text_right(
                    canvas,
                    fonts,
                    &format!("{s_seen}/{s_all}"),
                    x + w,
                    y + h + 26,
                    17.0,
                    if s_seen == s_all && s_all > 0 { GOLD } else { MUTED },
                    true,
                );
            }
            if ui_state.0 && ui_state.2 == index {
                title_menu::pointer(canvas, (x as f32 - 12.0, (y + h / 2) as f32), 18.0);
            }
        }
        if count == 0 {
            ui::text_mid(
                canvas,
                fonts,
                t("Aún no hay nada aquí.", "Nothing here yet."),
                694,
                300,
                22.0,
                MUTED,
                false,
            );
        }

        // ── The keys.
        let hints: &[(&str, &str)] = if ui_state.0 {
            &[
                ("←↑↓→", t("elegir", "choose")),
                ("ENTER", t("ver", "view")),
                ("Q E", t("página", "page")),
                ("ESC", t("volver", "back")),
            ]
        } else {
            &[
                ("↑↓", t("elegir", "choose")),
                ("→", t("entrar", "enter")),
                ("ESC", t("volver", "back")),
            ]
        };
        ui::key_hints(canvas, fonts, 80, 1010, hints);
    }

    fn paint_gallery_still(&mut self, n: usize) {
        let Some(entry) = self.gallery.entries.get(n).cloned() else {
            self.gallery_ui.viewer = None;
            return;
        };
        self.canvas.clear(0xff0b0710);
        if let Some(b) = self.art.original(&entry.asset) {
            self.canvas.blit_rgba(0, 0, b);
        }
        if self.ui_hidden {
            return;
        }
        let light = ui::light_from_pixels(&self.canvas.pixels, self.canvas.pw, self.canvas.ph);
        let accent = ui::accent(light);
        let language = self.localizer.language();
        let t = |es, en| t(language, es, en);
        let total = self.gallery.entries.len();
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let canvas = &mut self.canvas;
        // The line, in the box's manner: a slanted ink plate, the speaker on a plate of the light.
        ui::plate(canvas, 70.0, 868.0, 1380.0, 150.0, 34.0, 0xb4000000);
        ui::plate(canvas, 60.0, 858.0, 1380.0, 150.0, 34.0, 0xec100a16);
        let speaker = if entry.speaker.is_empty() {
            entry.room_name.clone()
        } else {
            format!("{} · {}", entry.speaker, entry.room_name)
        };
        let sw = fonts.text_width(&speaker, 20.0, true) + 60;
        ui::plate(canvas, 92.0, 832.0, sw as f32, 46.0, 14.0, ui::argb(0xff, accent));
        ui::text_mid(canvas, fonts, &speaker, 118, 855, 20.0, ui::ink_on(accent), true);
        canvas.paragraph(fonts, &entry.text, 116, 900, 1260, 34, 25.0, INK, false);
        ui::text_right(
            canvas,
            fonts,
            &format!("{} / {total}", n + 1),
            1850,
            60,
            24.0,
            0xffffffff,
            true,
        );
        ui::key_hints(
            canvas,
            fonts,
            1478,
            970,
            &[("←→", t("pasar", "browse")), ("ESC", t("volver", "back"))],
        );
    }

    fn paint_gallery_scene(&mut self) {
        let (album, scene, shot) = match &self.gallery_ui.viewer {
            Some(Viewer::Scene {
                album, scene, shot, ..
            }) => (*album, *scene, *shot),
            _ => return,
        };
        self.canvas.clear(0xff07040b);
        let place = self.title_background_current.min(title_menu::PLACES.len() - 1);
        let light = self.title_light(place);
        let accent = ui::accent(light);
        ui::halftone(
            &mut self.canvas,
            (960.0, 540.0, 960.0, 540.0),
            18.0,
            4.0,
            (1920.0, 1080.0),
            900.0,
            ui::argb(0x26, accent),
        );
        // The shot playing, 1:1 in the middle-right.
        let frame_w = if let Some(Viewer::Scene { reel: Some(reel), .. }) = &mut self.gallery_ui.viewer {
            reel.frame().map(|b| {
                let (x, y) = (1180 - b.w as i32 / 2, (1080 - b.h as i32) / 2);
                self.canvas.blit_rgba(x, y, b);
                b.w as i32
            })
        } else {
            None
        };
        if self.ui_hidden {
            return;
        }
        let language = self.localizer.language();
        let t = |es, en| t(language, es, en);
        let Some(shots) = self.gallery_ui.shots(album, scene) else {
            return;
        };
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let canvas = &mut self.canvas;
        if let Some(w) = frame_w {
            // A frame of the accent round the picture.
            let (x, y) = (1180 - w / 2, (1080 - w.min(1080)) / 2);
            canvas.round_outline(x - 4, y.max(0) - 4, w + 8, w.min(1080) + 8, 0, 3.0, ui::argb(0xff, accent));
        }
        // The band of the scene on the left: album, name, shots.
        ui::plate(canvas, -40.0, 0.0, 690.0, 1080.0, 90.0, 0xe60b0710);
        ui::plate(canvas, 600.0, 0.0, 10.0, 1080.0, 1.5, ui::argb(0xff, accent));
        ui::tag(
            canvas,
            fonts,
            44,
            92,
            t(ALBUMS[album].es, ALBUMS[album].en),
            16.0,
            ui::ABYSS,
            ui::BONE,
        );
        let def = &ALBUMS[album].scenes[scene];
        let name = t(def.es, def.en);
        let size = ui::fit_size(fonts, name, 520, 62.0, false);
        canvas.display_text(fonts, name, 44 + 4, 134 + 4, size, 0xff000000);
        canvas.display_text(fonts, name, 44, 134, size, 0xffffffff);
        ui::text_mid(
            canvas,
            fonts,
            &format!(
                "{} · {}/{}",
                t("TOMAS", "SHOTS"),
                shots.seen(),
                shots.shots.len()
            ),
            46,
            326,
            17.0,
            GOLD,
            true,
        );
        for (i, (_, seen)) in shots.shots.iter().enumerate() {
            let r = shot_rect(i);
            let (x, y, w, h) = (r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
            let on = i == shot;
            let over = *seen && inside(r, self.mouse.0, self.mouse.1);
            ui::plate(canvas, x + 4.0, y + 4.0, w, h, 10.0, 0x99000000);
            let face = if on {
                ui::argb(0xff, accent)
            } else if over {
                ui::argb(0xff, ui::mix(0x2a1a36, accent, 0.3))
            } else if *seen {
                0xff1c1424
            } else {
                0xff0e0a12
            };
            ui::plate(canvas, x, y, w, h, 10.0, face);
            if *seen {
                ui::text_center(
                    canvas,
                    fonts,
                    &format!("{}", i + 1),
                    r.0 + r.2 / 2,
                    r.1 + r.3 / 2,
                    20.0,
                    if on { ui::ink_on(accent) } else { INK },
                    true,
                );
            } else {
                ui::icon(canvas, Icon::Lock, r.0 + r.2 / 2, r.1 + r.3 / 2, 18, 0xff5a4d63, 0xff0e0a12);
            }
        }
        let rows = shots.shots.len().div_ceil(SHOT_COLS) as i32;
        canvas.paragraph(
            fonts,
            t(
                "Las tomas con candado se abren al verlas jugando la escena.",
                "Locked shots open once you see them playing the scene.",
            ),
            46,
            380 + rows * 58,
            520,
            26,
            16.0,
            MUTED,
            false,
        );
        ui::key_hints(
            canvas,
            fonts,
            44,
            1010,
            &[("←→", t("toma", "shot")), ("ESC", t("volver", "back"))],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scene_has_its_shots_and_its_cover() {
        let root = crate::assets_root();
        for album in &ALBUMS[1..] {
            for scene in album.scenes {
                let dir = root.join(scene.dir);
                let shots = std::fs::read_dir(&dir)
                    .unwrap_or_else(|_| panic!("{}", dir.display()))
                    .flatten()
                    .filter(|e| crate::exploration3d::clip_video::is_shot(&e.path()))
                    .count();
                assert!(shots > 0, "{} has no shots", scene.dir);
                assert!(
                    root.join(scene_thumb_key(scene.dir)).is_file(),
                    "{} has no cover (tools/gallery_thumbs.py)",
                    scene.dir
                );
            }
        }
    }

    #[test]
    fn the_keys_walk_the_albums_and_the_cards_and_esc_goes_back() {
        let mut app = App::default();
        app.screen = crate::Screen::Title;
        app.open_gallery(crate::Screen::Title);
        assert_eq!(app.screen, crate::Screen::Gallery);
        assert_eq!((app.gallery_ui.album, app.gallery_ui.on_grid), (0, false));
        app.gallery_key(KeyCode::ArrowDown);
        app.gallery_key(KeyCode::ArrowDown);
        assert_eq!(app.gallery_ui.album, 2);
        app.gallery_key(KeyCode::ArrowRight);
        assert!(app.gallery_ui.on_grid);
        app.gallery_key(KeyCode::ArrowRight);
        assert_eq!(app.gallery_ui.tile, 1);
        app.gallery_key(KeyCode::ArrowDown);
        assert_eq!(app.gallery_ui.tile, 5);
        app.gallery_key(KeyCode::ArrowLeft);
        app.gallery_key(KeyCode::ArrowLeft);
        assert!(!app.gallery_ui.on_grid, "left from the first column goes back to the list");
        // A scene opened for a capture plays its shot; Esc closes it, then leaves.
        app.gallery_ui.album = 4;
        app.gallery_capture_card(0, Some(0));
        assert!(app.gallery_ui.playing());
        app.gallery_key(KeyCode::Escape);
        assert!(app.gallery_ui.viewer.is_none());
        assert_eq!(app.screen, crate::Screen::Gallery);
        app.gallery_key(KeyCode::Escape);
        assert_eq!(app.screen, crate::Screen::Title);
    }

    #[test]
    fn the_way_in_comes_first_and_the_calm_last() {
        let mut names = vec!["relax", "climax", "bucle", "intro", "bucle_rapido"];
        names.sort_by_key(|n| shot_rank(n));
        assert_eq!(names, ["intro", "bucle", "bucle_rapido", "climax", "relax"]);
    }
}
