//! Versioned run state. Regenerate the immutable blueprint, then validate every
//! mutable record before applying any of them. Never trust saved actor indices.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub seed: u64,
    pub rules_hash: u64,
    pub config: Config,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub checkpoint: Vec3,
    pub health: f32,
    pub potions: u32,
    #[serde(default)]
    pub herbs: u32,
    #[serde(default)]
    pub unbanked_potions: u32,
    #[serde(default)]
    pub unbanked_herbs: u32,
    pub gold: u32,
    pub sigils: [u8; 3],
    pub crest: bool,
    pub won: bool,
    pub actors: Vec<Actor>,
    #[serde(default)]
    pub slime: Option<super::slime::Slime>,
    #[serde(default)]
    pub groves: Vec<super::slime::Slime>,
    pub opened: BTreeSet<usize>,
    pub visited: BTreeSet<usize>,
    pub torch: bool,
    #[serde(default)]
    pub materials: std::collections::BTreeMap<String, u32>,
    #[serde(default)]
    pub relic_sword: bool,
    #[serde(default)]
    pub unbanked_relic_sword: bool,
}
impl Dungeon {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: GENERATOR_VERSION,
            seed: self.seed(),
            rules_hash: self.map.rules_hash,
            config: self.map.config.clone(),
            // saved down the shaft: she comes back by its winch, in the mine
            position: self
                .pozo
                .as_ref()
                .map_or(self.controller.position, |p| p.came_from.0),
            yaw: self.controller.yaw,
            pitch: self.controller.pitch,
            checkpoint: self.checkpoint,
            health: self.health,
            potions: self.potions,
            herbs: self.herbs,
            unbanked_potions: self.unbanked_potions,
            unbanked_herbs: self.unbanked_herbs,
            gold: self.gold,
            sigils: self.sigils,
            crest: self.crest,
            won: self.won,
            // The Heart's summons are never saved.
            actors: self.actors[..self.base_actors].to_vec(),
            slime: self.slime.clone(),
            groves: self.groves.clone(),
            opened: self.opened.clone(),
            visited: self.visited.clone(),
            torch: self.torch,
            materials: self.materials.clone(),
            relic_sword: self.relic_sword,
            unbanked_relic_sword: self.unbanked_relic_sword,
        }
    }
    pub fn restore(saved: &Snapshot) -> Result<Self, String> {
        if saved.version != GENERATOR_VERSION {
            return Err("Expedicion guardada con otra version del generador.".into());
        }
        let mut run = Self::try_generate(saved.seed, saved.config.clone())?;
        if saved.rules_hash != run.map.rules_hash {
            return Err(
                "Las reglas de la expedicion han cambiado; no se aplico el guardado.".into(),
            );
        }
        let finite = |v: Vec3| v.to_array().iter().all(|x| x.is_finite());
        if !finite(saved.position)
            || !finite(saved.checkpoint)
            || !saved.yaw.is_finite()
            || !saved.pitch.is_finite()
            || !(-1.48..=1.48).contains(&saved.pitch)
            || !saved.health.is_finite()
            || !(0.0..=100.0).contains(&saved.health)
            || saved.unbanked_potions > 100_000
            || saved.unbanked_herbs > 100_000
            || saved.sigils.iter().any(|s| *s > 3)
            || saved.actors.len() != run.actors.len()
            || saved
                .slime
                .as_ref()
                .is_some_and(|slime| !slime.valid(&run.map))
            || saved.opened.iter().any(|i| *i >= run.map.connections.len())
            || saved.visited.iter().any(|i| *i >= run.map.rooms.len())
            || run
                .map
                .capsule_floor(saved.position, PLAYER_RADIUS)
                .is_none()
            || run
                .map
                .capsule_floor(saved.checkpoint, PLAYER_RADIUS)
                .is_none()
        {
            return Err("Estado de expedicion invalido.".into());
        }
        for (i, a) in saved.actors.iter().enumerate() {
            if a.spawn != i
                || !finite(a.position)
                || !a.health.is_finite()
                || a.health > run.map.spawns[i].health
                || a.damage_hits > 3
                || !a.hit_reaction.is_finite()
                || !(0.0..=0.42).contains(&a.hit_reaction)
                || !a.cooldown.is_finite()
                || !finite(a.attack_target)
                || !a.yaw.is_finite()
                || !(0.0..=0.38).contains(&a.stagger)
                || a.attack.is_some_and(|swing| {
                    let spawn = &run.map.spawns[i];
                    !swing.t.is_finite()
                        || swing.t < 0.0
                        || !finite(swing.cast_point)
                        || swing.index
                            >= super::combat::moves(&spawn.definition, spawn.kind, spawn.id).len()
                })
                || !a.chest_open_progress.is_finite()
                || !(0.0..=1.0).contains(&a.chest_open_progress)
                || (run.map.spawns[i].kind != SpawnKind::Chest
                    && (a.chest_open_progress > 0.0 || a.chest_opening))
                || (a.chest_opening && (a.used || a.chest_open_progress >= 1.0))
                || a.cooldown < 0.0
                || run.map.capsule_floor(a.position, 0.32).is_none()
            {
                return Err(format!("Actor guardado {i} invalido."));
            }
        }
        run.controller = Controller::new(saved.position);
        run.controller.yaw = saved.yaw;
        run.controller.pitch = saved.pitch;
        run.checkpoint = saved.checkpoint;
        run.health = saved.health;
        run.potions = saved.potions;
        run.herbs = saved.herbs;
        run.unbanked_potions = saved.unbanked_potions;
        run.unbanked_herbs = saved.unbanked_herbs;
        run.gold = saved.gold;
        run.sigils = saved.sigils;
        for index in 0..3 {
            let angle = saved.sigils[index] as f32 * std::f32::consts::FRAC_PI_2;
            run.rune_start_angle[index] = angle;
            run.rune_target_angle[index] = angle;
        }
        run.seal = saved.sigils == [1, 2, 3];
        run.crest = saved.crest;
        run.won = saved.won;
        run.actors = saved.actors.clone();
        run.slime = saved.slime.clone().or(run.slime);
        // Older saves have no grove slimes: keep the freshly placed ones.
        if !saved.groves.is_empty() && saved.groves.iter().all(|slime| slime.valid(&run.map)) {
            run.groves = saved.groves.clone();
        }
        // Chests, rest fonts, seals and markers never move: always take their
        // place from the regenerated map, so older saves follow layout tweaks.
        for (actor, spawn) in run.actors.iter_mut().zip(&run.map.spawns) {
            if spawn.health == 0.0 {
                actor.position = spawn.position;
            }
        }
        run.opened = saved.opened.clone();
        run.visited = saved.visited.clone();
        run.torch = saved.torch;
        run.relic_sword = saved.relic_sword;
        run.materials = saved
            .materials
            .iter()
            .filter(|(id, count)| {
                **count < 100_000 && super::MATERIALS.iter().any(|(m, _)| m == id)
            })
            .map(|(id, count)| (id.clone(), *count))
            .collect();
        run.unbanked_relic_sword = saved.unbanked_relic_sword;
        run.message("Expedicion restaurada.");
        Ok(run)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_round_trip_and_corruption() {
        let mut run = Dungeon::generate(42);
        run.gold = 27;
        run.sigils = [1, 2, 3];
        let s: Snapshot =
            serde_json::from_slice(&serde_json::to_vec(&run.snapshot()).unwrap()).unwrap();
        let restored = Dungeon::restore(&s).unwrap();
        assert_eq!(restored.snapshot(), s);
        assert!((restored.rune_angle(0) - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
        assert!((restored.rune_angle(1) - std::f32::consts::PI).abs() < 0.0001);
        let mut bad = s.clone();
        bad.actors[0].spawn = usize::MAX;
        assert!(Dungeon::restore(&bad).is_err());
        let mut bad = s;
        bad.version = 1;
        assert!(Dungeon::restore(&bad).is_err());
    }

    #[test]
    fn chest_opens_before_reward_and_can_only_be_claimed_once() {
        let mut run = Dungeon::generate(42);
        let chest_index = run
            .actors
            .iter()
            .position(|actor| run.map.spawns[actor.spawn].kind == SpawnKind::Chest)
            .expect("seed has a treasure chest");
        let chest_position = run.actors[chest_index].position;
        for (index, actor) in run.actors.iter_mut().enumerate() {
            if index != chest_index {
                actor.used = true;
            }
        }
        run.controller.position = chest_position + Vec3::new(0.0, 0.0, 0.9);
        assert_eq!(run.nearby_actor(), Some(chest_index));

        let chest_spawn = run.map.spawns[run.actors[chest_index].spawn].clone();
        run.interact();
        assert!(run.actors[chest_index].chest_opening);
        assert_eq!(run.gold, 0);
        assert_eq!(run.potions, 3);

        for _ in 0..20 {
            run.update(Input::default(), 0.05);
        }
        assert_eq!(run.actors[chest_index].chest_open_progress, 1.0);
        assert!(!run.actors[chest_index].chest_opening);
        assert!(run.prompt().contains("Recoger"));

        run.interact();
        assert!(run.actors[chest_index].used);
        assert_eq!(run.gold, chest_spawn.gold);
        assert_eq!(run.potions, 4);
        let herbs = chest_herb_count(run.seed(), chest_spawn.id);
        assert_eq!(run.herbs, herbs);
        assert_eq!(run.unbanked_potions, 1);
        assert_eq!(run.unbanked_herbs, herbs);
        let restored = Dungeon::restore(&run.snapshot()).unwrap();
        assert_eq!(restored.unbanked_potions, 1);
        assert_eq!(restored.unbanked_herbs, herbs);
        run.interact();
        assert_eq!(run.gold, chest_spawn.gold);
        assert_eq!(run.potions, 4);
        assert_eq!(run.herbs, herbs);
        assert_eq!(run.unbanked_potions, 1);
        assert_eq!(run.unbanked_herbs, herbs);
        assert_eq!(run.take_acquired_items(), (1, herbs));
        assert_eq!(run.take_acquired_items(), (0, 0));
    }

    #[test]
    fn fixed_props_follow_the_map_when_an_old_save_is_restored() {
        let run = Dungeon::generate(42);
        let mut snapshot = run.snapshot();
        let index = run
            .map
            .spawns
            .iter()
            .position(|spawn| spawn.kind == SpawnKind::Rest)
            .expect("seed has a sanctuary");
        // An older layout stored the font elsewhere in the same room.
        let room = &run.map.rooms[run.map.spawns[index].room];
        snapshot.actors[index].position = room.center;
        let restored = Dungeon::restore(&snapshot).unwrap();
        assert_eq!(
            restored.actors[index].position,
            run.map.spawns[index].position
        );
    }

    #[test]
    fn chest_animation_state_survives_a_save_round_trip() {
        let mut run = Dungeon::generate(42);
        let chest = run
            .actors
            .iter_mut()
            .find(|actor| run.map.spawns[actor.spawn].kind == SpawnKind::Chest)
            .expect("seed has a treasure chest");
        chest.chest_open_progress = 0.37;
        chest.chest_opening = true;

        let restored = Dungeon::restore(&run.snapshot()).unwrap();
        let restored_chest = restored
            .actors
            .iter()
            .find(|actor| restored.map.spawns[actor.spawn].kind == SpawnKind::Chest)
            .unwrap();
        assert_eq!(restored_chest.chest_open_progress, 0.37);
        assert!(restored_chest.chest_opening);
    }

    #[test]
    fn chest_herb_rewards_are_seeded_and_old_snapshots_default_to_zero() {
        assert_eq!(chest_herb_count(42, 7), chest_herb_count(42, 7));
        assert!((1..=2).contains(&chest_herb_count(42, 7)));

        let mut run = Dungeon::generate(42);
        run.herbs = 2;
        let snapshot = run.snapshot();
        assert_eq!(Dungeon::restore(&snapshot).unwrap().herbs, 2);

        let mut old = serde_json::to_value(snapshot).unwrap();
        old.as_object_mut().unwrap().remove("herbs");
        old.as_object_mut().unwrap().remove("unbanked_potions");
        old.as_object_mut().unwrap().remove("unbanked_herbs");
        let restored: Snapshot = serde_json::from_value(old).unwrap();
        assert_eq!(restored.herbs, 0);
        assert_eq!(restored.unbanked_potions, 0);
        assert_eq!(restored.unbanked_herbs, 0);
    }
}
