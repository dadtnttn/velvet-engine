//! Lo que se pinta sobre el mundo: los medallones de F, los bocadillos de quien habla, el pensamiento de la Reina, y las vistas de logros y del armario.
use super::*;

/// Keep an interaction prompt solid while it is usable, then dissolve it
/// only across the outer approach band.  Fading across the whole show radius
/// made a nearby keycap look translucent before the player could activate it.
pub(super) fn interaction_prompt_fade(
    distance: f32,
    show_radius: f32,
    interact_radius: f32,
) -> f32 {
    if distance <= interact_radius {
        return 1.0;
    }
    let approach =
        ((show_radius - distance) / (show_radius - interact_radius).max(0.001)).clamp(0.0, 1.0);
    approach * approach * (3.0 - 2.0 * approach)
}

/// Draw one villa building's `F` affordance.  The nearest footprint is chosen
/// by the caller so a crossroads never fills with seven overlapping keycaps.
/// Each cap still carries the footprint's stable VN ID in the interaction
/// state, making the visual marker and future story hook share one anchor.
/// The F prompt of one villa building: its own icon, verb and name.
pub(super) fn building_prompt(building: village::BuildingAnchor) -> Prompt {
    let (icon, verb) = match building.id {
        "villa_tienda" => (Icon::Shop, "Comerciar"),
        "villa_taberna" => (Icon::Tavern, "Entrar a beber"),
        "villa_banos_termales" => (Icon::Bath, "Darse un baño"),
        "villa_consejo" => (Icon::Council, "Visitar al burgomaestre"),
        "villa_burdel" => (Icon::Heart, "Entrar"),
        "villa_casa_aldeanos_a" | "villa_casa_aldeanos_b" => (Icon::House, "Llamar a la puerta"),
        _ => (Icon::Door, "Entrar"),
    };
    // A door with nothing behind it yet says so before it is tried.
    if village::in_development(building.id).is_some() {
        return Prompt::new(icon, verb).detail(format!("{} · En desarrollo", building.label));
    }
    Prompt::new(icon, verb).detail(building.label)
}

pub(super) fn push_villa_building_prompt(anchor: village::BuildingAnchor, distance: f32) {
    // Over the door when the house has one facing a street, else its centre.
    let (base, out) =
        village::door_anchor(anchor.footprint_index).unwrap_or((anchor.position, Vec3::ZERO));
    push_prompt_medallion(
        building_prompt(anchor).icon,
        base + out * 0.45,
        2.05,
        distance,
        village::BUILDING_SHOW_RADIUS,
        village::BUILDING_INTERACT_RADIUS,
    );
}

thread_local! {
    /// The places to go in this frame, as the zones note them: (icon, world point, fade, ready).
    static MARKERS: std::cell::RefCell<Vec<(Icon, Vec3, f32, bool)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Forgets the places noted so far (a new frame begins).
pub(super) fn take_world_markers_raw() -> usize {
    MARKERS.with(|m| {
        let mut m = m.borrow_mut();
        let n = m.len();
        m.clear();
        n
    })
}

/// The places to go in noted this frame, on the screen (and the list emptied for the next one).
pub(super) fn take_world_markers(camera: Camera3D) -> Vec<map_overlay::WorldMarker> {
    MARKERS.with(|m| {
        m.borrow_mut()
            .drain(..)
            .filter_map(|(icon, at, fade, ready)| {
                let (x, y) = project_npc_anchor(camera, at)?;
                Some(map_overlay::WorldMarker {
                    x,
                    y,
                    icon,
                    fade,
                    ready,
                })
            })
            .collect()
    })
}

/// A place to go in, `lift` metres over `base`: noted for the screen, where it fades in over the
/// approach and lights up once the Queen is in reach.
pub(super) fn push_prompt_medallion(
    icon: Icon,
    base: Vec3,
    lift: f32,
    distance: f32,
    show_radius: f32,
    interact_radius: f32,
) {
    if distance > show_radius {
        return;
    }
    let fade = interaction_prompt_fade(distance, show_radius, interact_radius);
    if fade <= 0.01 {
        return;
    }
    // Drawn on the screen in the interface's manner (`map_overlay::WorldMarker`), not as a model
    // in the scene.
    let ready = distance <= interact_radius;
    MARKERS.with(|m| {
        m.borrow_mut()
            .push((icon, base + Vec3::Y * lift, fade, ready))
    });
}

pub(super) fn project_npc_anchor(camera: Camera3D, position: Vec3) -> Option<(f32, f32)> {
    let view_position = camera.view_matrix().transform_point3(position);
    if !view_position.z.is_finite() || view_position.z >= -camera.near {
        return None;
    }
    let ndc = camera
        .view_projection(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32)
        .transform_point3(position);
    if !ndc.x.is_finite()
        || !ndc.y.is_finite()
        || !ndc.z.is_finite()
        || ndc.z < 0.0
        || ndc.z > 1.0
        || ndc.x < -1.20
        || ndc.x > 1.20
        || ndc.y < -1.20
        || ndc.y > 1.20
    {
        return None;
    }
    Some((
        (ndc.x * 0.5 + 0.5) * WINDOW_WIDTH as f32,
        (1.0 - (ndc.y * 0.5 + 0.5)) * WINDOW_HEIGHT as f32,
    ))
}

/// The Queen's own thought, floating over her head for a moment.
pub(super) fn queen_thought_bubble(
    camera: Camera3D,
    player: Vec3,
    nature_mode: bool,
    map_open: bool,
    wardrobe_open: bool,
    thought: Option<(&'static str, f32)>,
) -> Option<map_overlay::NpcSpeechBubble> {
    let (text, opacity) = thought?;
    if map_open || wardrobe_open || opacity <= 0.01 {
        return None;
    }
    let ground = if nature_mode {
        nature_walk_surface_height(player.x, player.z)
    } else {
        player.y
    };
    let (x, y) = project_npc_anchor(camera, Vec3::new(player.x, ground + 3.75, player.z))?;
    Some(map_overlay::NpcSpeechBubble {
        x,
        y,
        speaker: "Reina",
        text,
        opacity,
        accent: [0.90, 0.46, 0.64, 1.0],
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn npc_speech_bubbles(
    camera: Camera3D,
    player: Vec3,
    elapsed: f32,
    nature_mode: bool,
    village_entered: bool,
    map_open: bool,
    wardrobe_open: bool,
    queen_thought: Option<(&'static str, f32)>,
) -> Vec<map_overlay::NpcSpeechBubble> {
    let mut bubbles = npc_speech_bubbles_of_others(
        camera,
        player,
        elapsed,
        nature_mode,
        village_entered,
        map_open,
        wardrobe_open,
    );
    bubbles.extend(queen_thought_bubble(
        camera,
        player,
        nature_mode,
        map_open,
        wardrobe_open,
        queen_thought,
    ));
    bubbles
}

thread_local! {
    /// What the hypnotist blurts out instead of his usual lines, and until when.
    static MAGE_SAYS: std::cell::Cell<Option<(&'static str, f32)>> = const { std::cell::Cell::new(None) };
}

/// The hypnotist says `text` over his head until `until` (the world's clock).
pub(super) fn mage_says(text: &'static str, until: f32) {
    MAGE_SAYS.with(|m| m.set(Some((text, until))));
}

pub(super) fn npc_speech_bubbles_of_others(
    camera: Camera3D,
    player: Vec3,
    elapsed: f32,
    nature_mode: bool,
    village_entered: bool,
    map_open: bool,
    wardrobe_open: bool,
) -> Vec<map_overlay::NpcSpeechBubble> {
    if map_open || wardrobe_open {
        return Vec::new();
    }
    if nature_mode && village_entered {
        // The village currently ships as an environment-only world. Keep the
        // outdoor mage bubble out of the settlement until village NPCs land.
        return Vec::new();
    }
    if nature_mode {
        // The outdoor mage keeps the old proximity-gated behaviour.
        let distance = mage_distance(player);
        if distance > NPC_COMMENT_SHOW_RADIUS {
            return Vec::new();
        }
        let blurt = MAGE_SAYS
            .with(|m| m.get())
            .filter(|(_, until)| elapsed < *until)
            .map(|(text, _)| (text, 1.0));
        let (text, accent, line_opacity) = match blurt {
            Some((text, opacity)) => (text, [0.62, 0.44, 0.96, 1.0], opacity),
            None => {
                let Some((line, line_opacity)) = active_npc_comment(elapsed) else {
                    return Vec::new();
                };
                (line.text, line.accent, line_opacity)
            }
        };
        let mage = mage_position();
        let ground_y = nature_walk_surface_height(mage.x, mage.z);
        let bob = (elapsed * 2.2).sin() * 0.035;
        let Some((x, y)) =
            project_npc_anchor(camera, Vec3::new(mage.x, ground_y + 3.62 + bob, mage.z))
        else {
            return Vec::new();
        };
        let distance_opacity =
            interaction_prompt_fade(distance, NPC_COMMENT_SHOW_RADIUS, MAGE_INTERACT_RADIUS);
        return vec![map_overlay::NpcSpeechBubble {
            x,
            y,
            speaker: "Mago",
            text,
            opacity: line_opacity * distance_opacity,
            accent,
        }];
    }

    // In the castle its own maids speak (castle3d::State::bubbles).
    Vec::new()
}

impl App {
    /// Reports something she did (see `data/achievements/README.md` for the events). Unlocks
    /// whatever it completes, hands over its gold and items, queues the notice, and saves.
    pub(crate) fn achievement_event(&mut self, event: &str) {
        self.achievements.record(event);
        self.check_achievements();
    }

    /// Unlocks what holds now (gold and items can complete achievements too).
    pub(crate) fn check_achievements(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let catalog = achievements::catalog();
        let unlocked = {
            let world = achievements::World {
                gold: self.gold,
                inventory: &self.inventory,
            };
            self.achievements.evaluate(catalog, &world, now)
        };
        if unlocked.is_empty() {
            return;
        }
        for u in unlocked {
            let mut gains = Vec::new();
            for reward in &u.rewards {
                match reward {
                    achievements::Reward::Gold { amount } => {
                        self.gold = self.gold.saturating_add(*amount);
                        gains.push(format!("+{amount} oro"));
                    }
                    achievements::Reward::Item { id, count } => {
                        *self.inventory.entry(id.clone()).or_default() += *count;
                        let name = shop::catalog()
                            .products
                            .iter()
                            .find(|p| &p.id == id)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| id.clone());
                        gains.push(format!("{name} ×{count}"));
                    }
                    achievements::Reward::Outfit { id } => {
                        let name = catalog
                            .outfits
                            .iter()
                            .find(|o| &o.id == id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| id.clone());
                        gains.push(format!("Atuendo: {name}"));
                    }
                    achievements::Reward::Flag { .. } => {}
                }
            }
            let detail = if gains.is_empty() {
                u.title.clone()
            } else {
                format!("{} · {}", u.title, gains.join(", "))
            };
            eprintln!("[3d] achievement unlocked: {} ({})", u.id, detail);
            self.achievement_toasts.push_back(
                Prompt::new(achievements::icon(&u.icon), "¡Logro desbloqueado!").detail(detail),
            );
        }
        // Items earned go into the backpack's grid like any other.
        let counts = self.inventory.clone();
        self.backpack.reconcile(&counts);
        self.persist_3d_state(true);
    }

    /// The achievements screen's content: every achievement (hidden ones veiled until earned) with
    /// what it gives, and counts of what she has done.
    pub(super) fn achievements_view(&self) -> map_overlay::AchievementsView {
        let catalog = achievements::catalog();
        let p = &self.achievements;
        let rows = catalog
            .achievements
            .iter()
            .map(|a| {
                let done = p.is_unlocked(&a.id);
                if a.hidden && !done {
                    return map_overlay::AchievementRow {
                        title: "???".into(),
                        detail: "Logro oculto".into(),
                        reward: String::new(),
                        done,
                        icon: None,
                    };
                }
                let reward = a
                    .rewards
                    .iter()
                    .filter_map(|r| match r {
                        achievements::Reward::Outfit { id } => Some(format!(
                            "Atuendo: {}",
                            catalog
                                .outfits
                                .iter()
                                .find(|o| &o.id == id)
                                .map_or(id.as_str(), |o| o.name.as_str())
                        )),
                        achievements::Reward::Item { id, count } => Some(format!(
                            "{} ×{count}",
                            shop::catalog()
                                .products
                                .iter()
                                .find(|q| &q.id == id)
                                .map_or(id.as_str(), |q| q.name.as_str())
                        )),
                        achievements::Reward::Gold { amount } => Some(format!("+{amount} oro")),
                        achievements::Reward::Flag { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                map_overlay::AchievementRow {
                    title: a.title.clone(),
                    detail: a.description.clone(),
                    reward,
                    done,
                    icon: Some(achievements::icon(&a.icon)),
                }
            })
            .collect();
        let places = p
            .counters
            .iter()
            .filter(|(k, v)| k.starts_with("visit:") && k.matches(':').count() == 1 && **v > 0)
            .count();
        let outfits = (0..wardrobe_outfit_count())
            .filter(|&s| p.outfit_available(catalog, s))
            .count();
        let stats = vec![
            ("Lugares visitados".to_owned(), places.to_string()),
            (
                "Veces en las termas".to_owned(),
                p.count("visit:termas").to_string(),
            ),
            (
                "Bajadas a la mina".to_owned(),
                p.count("visit:mina").to_string(),
            ),
            (
                "Charlas con el mago".to_owned(),
                p.count("talk:mago").to_string(),
            ),
            ("Compras".to_owned(), p.count("shop:buy").to_string()),
            (
                "Atuendos".to_owned(),
                format!("{outfits} de {}", wardrobe_outfit_count()),
            ),
            ("Oro".to_owned(), self.gold.to_string()),
        ];
        map_overlay::AchievementsView { rows, stats }
    }

    /// The wardrobe's outfits as the catalog names them, and which are still locked.
    pub(super) fn wardrobe_view(&self) -> map_overlay::WardrobeView {
        let catalog = achievements::catalog();
        let outfits = (0..wardrobe_outfit_count())
            .map(|slot| {
                catalog
                    .outfit_in_slot(slot)
                    .map(|o| {
                        let detail = match o.id.as_str() {
                            "capa" => "Para colarse en el burdel de noche",
                            _ => "",
                        };
                        (o.name.clone(), detail.to_owned())
                    })
                    .or_else(|| {
                        extra_outfit_specs()
                            .iter()
                            .find(|(s, ..)| *s == slot)
                            .map(|(_, n, _)| (n.clone(), "Modelo propio".to_owned()))
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let outfits = if outfits.iter().any(|(n, _)| n.is_empty()) {
            Vec::new()
        } else {
            outfits
        };
        let locked = (0..wardrobe_outfit_count())
            .map(|slot| !self.achievements.outfit_available(catalog, slot))
            .collect();
        // The Queen in each outfit: her own card picture (`vestidor_*`), else her status picture
        // (royal, green, bikini), when it exists.
        let pictures = (0..wardrobe_outfit_count())
            .map(|slot| {
                let (own, status) = match slot {
                    0 => ("ui/status/vestidor_reina", "ui/status/reina_status_0"),
                    1 => ("ui/status/vestidor_verde", "ui/status/reina_status_verde_0"),
                    2 => (
                        "ui/status/vestidor_bikini",
                        "ui/status/reina_status_bikini_0",
                    ),
                    3 => ("ui/status/vestidor_capa", "ui/status/vestidor_capa"),
                    _ => return None,
                };
                let assets = crate::exploration3d_state::data_dir().join("assets");
                let name = if assets.join(format!("{own}.png")).exists() {
                    own
                } else {
                    status
                };
                let path = assets.join(format!("{name}.png"));
                let (w, h) = image::image_dimensions(path).ok()?;
                Some(map_overlay::PortraitRef {
                    image: name.to_owned(),
                    crop: [0, 0, w, h],
                })
            })
            .collect();
        map_overlay::WardrobeView {
            outfits,
            locked,
            pictures,
        }
    }
}

pub(super) const ROOM_PLAYER_MARKER_Y: f32 = 0.075;
