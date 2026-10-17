//! The campaign's rules, as plain data and functions: the state, moves, captures, recruiting, the
//! economy (gold, food, the spark), the AI factions' turns, auto-resolve, and how a battle's
//! result changes the map. The whole state is serialisable (it rides in the game's saves).
use serde::{Deserialize, Serialize};

use crate::data::{
    adjacent, army_names, province, start_purse, unit, unit_power, Faction, Role, MAX_UNITS,
    PROVINCES, START_ARMIES,
};
use crate::noise::Rng;

const MP: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    #[serde(rename = "type")]
    pub ty: String,
    pub men: u32,
    pub max: u32,
}

pub fn new_unit(ty: &str, men: Option<u32>) -> Unit {
    let t = unit(ty);
    Unit {
        ty: ty.to_owned(),
        men: men.unwrap_or(t.men),
        max: t.men,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Army {
    pub id: String,
    pub faction: Faction,
    pub name: String,
    pub at: String,
    pub mp: u32,
    pub garrison: bool,
    pub units: Vec<Unit>,
    /// Where it marches, turn after turn (the Queen's armies), if anywhere.
    #[serde(default)]
    pub march: Option<String>,
}

impl Army {
    pub fn has(&self, ty: &str) -> bool {
        self.units.iter().any(|u| u.ty == ty)
    }

    pub fn power(&self) -> f32 {
        self.units
            .iter()
            .map(|u| unit_power(unit(&u.ty), u.men))
            .sum()
    }

    pub fn men(&self) -> u32 {
        self.units
            .iter()
            .filter(|u| unit(&u.ty).role != Role::Hero)
            .map(|u| u.men)
            .sum()
    }

    /// The unit that leads it (its hero, else its strongest).
    pub fn lead(&self) -> &Unit {
        self.units
            .iter()
            .find(|u| unit(&u.ty).role == Role::Hero)
            .unwrap_or_else(|| {
                self.units
                    .iter()
                    .max_by(|a, b| {
                        unit_power(unit(&a.ty), a.men).total_cmp(&unit_power(unit(&b.ty), b.men))
                    })
                    .expect("an army has units")
            })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Purse {
    pub gold: i32,
    pub food: i32,
    pub spark: i32,
    /// Turns the Queen still rests wounded (the spark grows at half).
    #[serde(default)]
    pub wounded: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvinceState {
    pub owner: Faction,
    pub lit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    Info,
    Good,
    Bad,
    Spark,
    Lore,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogLine {
    pub turn: u32,
    pub text: String,
    pub kind: LogKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingBattle {
    pub prov: String,
    pub from: String,
    pub attacker: String,
    pub defender: String,
    pub af: Faction,
    pub df: Faction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedMove {
    pub army: String,
    pub to: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Player,
    Ai,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub version: u32,
    pub seed: u32,
    pub turn: u32,
    pub phase: Phase,
    /// Gold, food and spark of the Queen, the Alliance and the rebels (by `Faction::index`).
    pub purses: [Purse; 3],
    /// Each province's owner and whether it is lit (by the index of `PROVINCES`).
    pub provinces: Vec<ProvinceState>,
    pub armies: Vec<Army>,
    pub log: Vec<LogLine>,
    pub queue: Vec<QueuedMove>,
    pub pending: Option<PendingBattle>,
    pub winner: Option<Faction>,
    pub next_id: u32,
    /// The intro was shown; Varrak woke.
    #[serde(default)]
    pub intro_seen: bool,
    #[serde(default)]
    pub varrak_awake: bool,
    #[serde(skip)]
    rng: Option<Rng>,
}

/// A battle's outcome, in the shape the battle mode and auto-resolve both give: the survivors of
/// side 0 (`player`) and side 1 (`enemy`) by their place in their army (`u0`, `u1`…).
#[derive(Clone, Debug, Default)]
pub struct BattleResult {
    pub winner: u8,
    pub player: Vec<Survivor>,
    pub enemy: Vec<Survivor>,
    pub auto: bool,
    pub label: String,
    pub varrak_awake: bool,
    pub queen_down: bool,
    pub note: String,
    pub lost: f32,
    pub time: f32,
}

#[derive(Clone, Debug)]
pub struct Survivor {
    pub id: String,
    pub ty: String,
    pub men0: u32,
    pub men: u32,
    pub kills: u32,
}

impl Campaign {
    /// A new campaign; `varrak_awake` when the story already woke him (the Peñascal is hers and he
    /// marches with her).
    pub fn new(seed: u32, varrak_awake: bool) -> Campaign {
        let purse = |f: Faction| {
            let (gold, food, spark) = start_purse(f);
            Purse {
                gold,
                food,
                spark,
                wounded: 0,
            }
        };
        let mut s = Campaign {
            version: 1,
            seed,
            turn: 1,
            phase: Phase::Player,
            purses: [
                purse(Faction::Reina),
                purse(Faction::Alianza),
                purse(Faction::Rebeldes),
            ],
            provinces: PROVINCES
                .iter()
                .map(|p| ProvinceState {
                    owner: p.owner,
                    lit: true,
                })
                .collect(),
            armies: START_ARMIES
                .iter()
                .map(|a| Army {
                    id: a.id.to_owned(),
                    faction: a.faction,
                    name: a.name.to_owned(),
                    at: a.at.to_owned(),
                    mp: MP,
                    garrison: false,
                    units: a.units.iter().map(|t| new_unit(t, None)).collect(),
                    march: None,
                })
                .collect(),
            log: Vec::new(),
            queue: Vec::new(),
            pending: None,
            winner: None,
            next_id: 1,
            intro_seen: false,
            varrak_awake: false,
            rng: None,
        };
        if varrak_awake {
            // He woke in the story: his pack broke up, his village came back to the Queen.
            s.armies.retain(|a| a.id != "r-manada");
            s.set_owner("penascal", Faction::Reina, true);
            if let Some(q) = s.armies.iter_mut().find(|a| a.id == "a-reina") {
                q.units.insert(1, new_unit("varrak_reina", Some(1)));
            }
            s.varrak_awake = true;
            s.note(
                "La Reina ha despertado a Varrak: el Peñascal vuelve a ser suyo. En el norte, el Nido sigue sin pagar tributo y la Alianza vigila el río.",
                LogKind::Lore,
            );
        } else {
            s.note(
                "La Reina ha despertado. La chispa no llega a todos: los clanes del norte se rebelan y la Alianza vigila el río.",
                LogKind::Lore,
            );
        }
        s
    }

    pub fn note(&mut self, text: impl Into<String>, kind: LogKind) {
        self.log.insert(
            0,
            LogLine {
                turn: self.turn,
                text: text.into(),
                kind,
            },
        );
        self.log.truncate(60);
    }

    fn rand(&mut self) -> f32 {
        let (seed, turn) = (self.seed, self.turn);
        self.rng
            .get_or_insert_with(|| {
                Rng::new(seed.wrapping_mul(977).wrapping_add(turn.wrapping_mul(131)))
            })
            .next()
    }

    pub fn purse(&self, f: Faction) -> &Purse {
        &self.purses[f.index()]
    }

    pub fn purse_mut(&mut self, f: Faction) -> &mut Purse {
        &mut self.purses[f.index()]
    }

    pub fn prov(&self, id: &str) -> &ProvinceState {
        &self.provinces[crate::data::province_index(id)]
    }

    fn prov_mut(&mut self, id: &str) -> &mut ProvinceState {
        let i = crate::data::province_index(id);
        &mut self.provinces[i]
    }

    pub fn owner(&self, id: &str) -> Faction {
        self.prov(id).owner
    }

    fn set_owner(&mut self, id: &str, f: Faction, lit: bool) {
        let p = self.prov_mut(id);
        p.owner = f;
        p.lit = lit;
    }

    // ------------------------------------------------------------------ queries
    pub fn army(&self, id: &str) -> Option<&Army> {
        self.armies.iter().find(|a| a.id == id)
    }

    pub fn army_mut(&mut self, id: &str) -> Option<&mut Army> {
        self.armies.iter_mut().find(|a| a.id == id)
    }

    pub fn armies_at<'a>(
        &'a self,
        prov: &'a str,
        faction: Option<Faction>,
    ) -> impl Iterator<Item = &'a Army> + 'a {
        self.armies
            .iter()
            .filter(move |a| a.at == prov && faction.map_or(true, |f| a.faction == f))
    }

    pub fn enemies_at<'a>(
        &'a self,
        prov: &'a str,
        faction: Faction,
    ) -> impl Iterator<Item = &'a Army> + 'a {
        self.armies
            .iter()
            .filter(move |a| a.at == prov && a.faction != faction)
    }

    pub fn queen_army(&self) -> Option<&Army> {
        self.armies
            .iter()
            .find(|a| a.faction == Faction::Reina && a.has("reina"))
    }

    /// Where an army can go this turn (adjacent, with movement left).
    pub fn reachable(&self, army: &Army) -> Vec<&'static str> {
        if army.mp == 0 || army.garrison {
            return Vec::new();
        }
        adjacent(&army.at)
            .iter()
            .copied()
            .filter(|p| {
                let full = self
                    .armies_at(p, Some(army.faction))
                    .any(|b| b.units.len() + army.units.len() > MAX_UNITS);
                !(full && self.enemies_at(p, army.faction).next().is_none())
            })
            .collect()
    }

    pub fn income(&self, faction: Faction) -> Income {
        let (mut gold, mut food, mut spark, mut upkeep, mut eat) =
            (0.0f32, 0.0f32, 0i32, 0i32, 0.0f32);
        for (k, p) in PROVINCES.iter().enumerate() {
            let st = &self.provinces[k];
            if st.owner != faction {
                continue;
            }
            let lit = if st.lit { 1.0 } else { 0.25 };
            gold += p.gold * lit;
            food += p.food * lit;
            spark += p.spark as i32;
        }
        for a in self.armies.iter().filter(|a| a.faction == faction) {
            for u in &a.units {
                let t = unit(&u.ty);
                let share = u.men as f32 / u.max.max(1) as f32;
                upkeep += (t.cost.gold as f32 * 0.06 * share).round() as i32;
                eat += t.cost.food as f32 * share;
            }
        }
        if faction == Faction::Reina {
            spark += 2;
            if self.purse(Faction::Reina).wounded > 0 {
                spark /= 2;
            }
        }
        Income {
            gold: gold.round() as i32,
            food: food.round() as i32,
            spark,
            upkeep,
            eat: eat.round() as i32,
            net_gold: (gold - upkeep as f32).round() as i32,
            net_food: (food - eat).round() as i32,
        }
    }

    /// The Queen's armies far from her and from the seal cost spark to keep lit.
    pub fn spark_drain(&self) -> Vec<String> {
        let queen_at = self.queen_army().map(|a| (a.id.clone(), a.at.clone()));
        self.armies
            .iter()
            .filter(|a| {
                a.faction == Faction::Reina && queen_at.as_ref().map_or(true, |(id, _)| *id != a.id)
            })
            .filter(|a| a.units.iter().any(|u| unit(&u.ty).spark))
            .filter(|a| {
                let near = hops(&a.at, "castillo")
                    .min(hops(&a.at, "cueva"))
                    .min(queen_at.as_ref().map_or(99, |(_, at)| hops(&a.at, at)));
                near > 1
            })
            .map(|a| a.id.clone())
            .collect()
    }

    // ---------------------------------------------------------------- recruiting
    pub fn recruit_options(&self, prov: &str, faction: Faction) -> Vec<RecruitOption> {
        let p = province(prov);
        let st = self.prov(prov);
        if st.owner != faction {
            return Vec::new();
        }
        let f = self.purse(faction);
        let here = self.armies_at(prov, Some(faction)).find(|a| !a.garrison);
        let armies = self
            .armies
            .iter()
            .filter(|a| a.faction == faction && !a.garrison)
            .count();
        p.recruits(faction)
            .iter()
            .map(|&ty| {
                let t = unit(ty);
                let why = if !st.lit {
                    "Sin chispa: enciende la provincia primero"
                } else if f.gold < t.cost.gold {
                    "Falta oro"
                } else if f.spark < t.cost.spark {
                    "Falta chispa"
                } else if here.is_some_and(|h| h.units.len() >= MAX_UNITS) {
                    "El ejército está lleno"
                } else if here.is_none() && armies >= 6 {
                    "Demasiados ejércitos"
                } else if self.enemies_at(prov, faction).next().is_some() {
                    "Hay enemigos aquí"
                } else {
                    ""
                };
                RecruitOption {
                    ty,
                    ok: why.is_empty(),
                    why,
                }
            })
            .collect()
    }

    /// Raises a regiment in `prov`; the army it joined (made if there was none).
    pub fn recruit(&mut self, prov: &str, ty: &str, faction: Faction) -> Option<String> {
        let opt = self
            .recruit_options(prov, faction)
            .into_iter()
            .find(|o| o.ty == ty)?;
        if !opt.ok {
            return None;
        }
        let t = unit(ty);
        {
            let f = self.purse_mut(faction);
            f.gold -= t.cost.gold;
            f.spark -= t.cost.spark;
        }
        let here = self
            .armies
            .iter()
            .position(|a| a.at == prov && a.faction == faction && !a.garrison);
        let idx = match here {
            Some(i) => i,
            None => {
                let names = army_names(faction);
                let id = format!("{}-{}", &faction.id()[..1], self.next_id);
                self.next_id += 1;
                let name = names[(self.next_id + self.turn) as usize % names.len()].to_owned();
                self.armies.push(Army {
                    id,
                    faction,
                    name,
                    at: prov.to_owned(),
                    mp: 0,
                    garrison: false,
                    units: Vec::new(),
                    march: None,
                });
                self.armies.len() - 1
            }
        };
        self.armies[idx].units.push(new_unit(ty, None));
        let (id, name) = (self.armies[idx].id.clone(), self.armies[idx].name.clone());
        if faction == Faction::Reina {
            self.note(
                if t.sing {
                    format!("{} se une a {} en {}.", t.call, name, province(prov).name)
                } else {
                    format!("{} se unen a {} en {}.", t.call, name, province(prov).name)
                },
                LogKind::Good,
            );
        }
        Some(id)
    }

    /// Lights a captured province with the Queen's spark: it pays in full and its people join.
    pub fn rekindle(&mut self, prov: &str) -> bool {
        let st = self.prov(prov);
        if st.owner != Faction::Reina || st.lit || self.purse(Faction::Reina).spark < 4 {
            return false;
        }
        self.purse_mut(Faction::Reina).spark -= 4;
        self.prov_mut(prov).lit = true;
        self.note(
            format!(
                "La chispa prende en {}: su gente vuelve a pensar.",
                province(prov).name
            ),
            LogKind::Spark,
        );
        let people = match prov {
            "penascal" => Some("trasgos"),
            "nido" => Some("arpias"),
            "marca" | "capital" => Some("orcos"),
            _ => None,
        };
        if let Some(people) = people {
            let t = unit(people);
            if let Some(here) = self
                .armies
                .iter_mut()
                .find(|a| a.at == prov && a.faction == Faction::Reina && !a.garrison)
            {
                if here.units.len() < MAX_UNITS {
                    here.units
                        .push(new_unit(people, Some((t.men as f32 * 0.6).round() as u32)));
                    let line = if t.sing {
                        format!("{} de {} se une a la Reina.", t.call, province(prov).name)
                    } else {
                        format!("{} de {} se unen a la Reina.", t.call, province(prov).name)
                    };
                    self.note(line, LogKind::Good);
                }
            }
        }
        true
    }

    // -------------------------------------------------------------------- moving
    /// Moves an army one step. A battle is left pending in `self.pending` for whoever must resolve
    /// it.
    pub fn move_army(&mut self, army_id: &str, to: &str) -> MoveResult {
        let Some(a) = self.army(army_id) else {
            return MoveResult::Blocked;
        };
        if !adjacent(&a.at).contains(&to) || a.mp == 0 {
            return MoveResult::Blocked;
        }
        let (from, faction) = (a.at.clone(), a.faction);
        let defender = self
            .enemies_at(to, faction)
            .filter(|b| !b.units.is_empty())
            .max_by(|x, y| x.power().total_cmp(&y.power()))
            .map(|d| (d.id.clone(), d.faction));
        if let Some((did, df)) = defender {
            self.army_mut(army_id).unwrap().mp = 0;
            let battle = PendingBattle {
                prov: to.to_owned(),
                from,
                attacker: army_id.to_owned(),
                defender: did,
                af: faction,
                df,
            };
            self.pending = Some(battle.clone());
            return MoveResult::Battle(battle);
        }
        let own = self.owner(to);
        // a capital defends itself with its garrison; a place lit by the Queen's spark, with its
        // people (taking it is a fight)
        let p = province(to);
        let defenders = self.defenders(to);
        if own != faction
            && !defenders.is_empty()
            && !self.armies.iter().any(|b| b.at == to && b.faction == own)
        {
            let militia = p.garrison.is_empty();
            let gid = format!("{}-{to}-{}", if militia { "m" } else { "g" }, self.next_id);
            self.next_id += 1;
            self.armies.push(Army {
                id: gid.clone(),
                faction: own,
                name: if militia {
                    format!("Milicia de {}", p.name)
                } else {
                    format!("Guarnición de {}", p.name)
                },
                at: to.to_owned(),
                mp: 0,
                garrison: true,
                units: defenders,
                march: None,
            });
            self.army_mut(army_id).unwrap().mp = 0;
            let battle = PendingBattle {
                prov: to.to_owned(),
                from,
                attacker: army_id.to_owned(),
                defender: gid,
                af: faction,
                df: own,
            };
            self.pending = Some(battle.clone());
            return MoveResult::Battle(battle);
        }
        {
            let a = self.army_mut(army_id).unwrap();
            a.at = to.to_owned();
            a.mp = if own != faction { 0 } else { a.mp - 1 };
        }
        if own != faction {
            self.capture(to, faction);
            return MoveResult::Capture;
        }
        MoveResult::Move
    }

    /// Who rises to defend `prov` when an enemy walks in and no army of its owner is there: a
    /// capital's garrison, or (in the Queen's lit lands) a militia of the people it raises, at
    /// about half strength. Empty: it falls without a fight.
    pub fn defenders(&self, prov: &str) -> Vec<Unit> {
        let p = province(prov);
        if !p.garrison.is_empty() {
            return p.garrison.iter().map(|t| new_unit(t, None)).collect();
        }
        let st = self.prov(prov);
        if st.owner != Faction::Reina || !st.lit {
            return Vec::new();
        }
        let mut kinds: Vec<&str> = p.recruits(Faction::Reina).iter().copied().take(2).collect();
        if kinds.is_empty() {
            kinds.push("trasgos");
        }
        if kinds.len() == 1 {
            kinds.push(kinds[0]);
        }
        kinds
            .into_iter()
            .map(|t| new_unit(t, Some((unit(t).men as f32 * 0.55).round() as u32)))
            .collect()
    }

    /// The shortest way (by steps) from where an army stands to `to`, `to` last; `None` when it is
    /// there already or cannot get there.
    pub fn route(&self, army: &Army, to: &str) -> Option<Vec<&'static str>> {
        if army.at == to {
            return None;
        }
        let start = PROVINCES.iter().find(|p| p.id == army.at)?.id;
        let mut prev: std::collections::HashMap<&'static str, &'static str> =
            std::collections::HashMap::new();
        let mut todo = std::collections::VecDeque::from([start]);
        while let Some(p) = todo.pop_front() {
            if p == to {
                let mut way = vec![p];
                let mut q = p;
                while let Some(&b) = prev.get(q) {
                    if b == start {
                        break;
                    }
                    way.push(b);
                    q = b;
                }
                way.reverse();
                return Some(way);
            }
            for &n in adjacent(p) {
                if n != start && !prev.contains_key(n) {
                    prev.insert(n, p);
                    todo.push_back(n);
                }
            }
        }
        None
    }

    /// How a march along `route` goes, turn by turn: for each step, the turn it is reached in (1 =
    /// this turn, with the moves the army has left). A step through her own, quiet lands costs one
    /// move; one into another's land or onto enemies ends that turn (it must be taken first).
    pub fn march_turns(&self, army: &Army, route: &[&str]) -> Vec<u32> {
        let mut turn = 1;
        let mut mp = army.mp;
        let mut out = Vec::with_capacity(route.len());
        for &p in route {
            if mp == 0 {
                turn += 1;
                mp = MP;
            }
            out.push(turn);
            let quiet =
                self.owner(p) == army.faction && self.enemies_at(p, army.faction).next().is_none();
            mp = if quiet { mp - 1 } else { 0 };
        }
        out
    }

    /// Joins army b into army a (same province).
    pub fn merge(&mut self, a_id: &str, b_id: &str) -> bool {
        let (Some(a), Some(b)) = (self.army(a_id), self.army(b_id)) else {
            return false;
        };
        if a.at != b.at || a.faction != b.faction || a.units.len() + b.units.len() > MAX_UNITS {
            return false;
        }
        let (units, mp) = (b.units.clone(), b.mp);
        let a = self.army_mut(a_id).unwrap();
        a.units.extend(units);
        a.mp = a.mp.min(mp);
        self.armies.retain(|x| x.id != b_id);
        true
    }

    pub fn capture(&mut self, prov: &str, faction: Faction) {
        let was = self.owner(prov);
        // the Queen's lands are lit; what she takes from others must be lit with her spark
        let lit = faction != Faction::Reina || province(prov).owner == Faction::Reina;
        self.set_owner(prov, faction, lit);
        // garrisons of the old owner go
        self.armies
            .retain(|g| !(g.garrison && g.at == prov && g.faction != faction));
        let f = faction.info();
        let line = if faction == Faction::Rebeldes {
            format!(
                "{} toman {} (antes de {}).",
                f.short,
                province(prov).name,
                was.info().short
            )
        } else {
            format!(
                "{} toma {} (antes de {}).",
                f.short,
                province(prov).name,
                was.info().short
            )
        };
        self.note(
            line,
            if faction == Faction::Reina {
                LogKind::Good
            } else {
                LogKind::Bad
            },
        );
        self.check_end();
    }

    fn check_end(&mut self) {
        if self.owner("capital") == Faction::Reina && self.winner.is_none() {
            self.winner = Some(Faction::Reina);
            self.note(
                "La capital caída vuelve a ser de la Reina. Las agujas violetas se encienden.",
                LogKind::Spark,
            );
        }
        if self.owner("castillo") != Faction::Reina && self.winner.is_none() {
            self.winner = Some(self.owner("castillo"));
            self.note(
                "El castillo ha caído. La Reina huye hacia la cueva del sello…",
                LogKind::Bad,
            );
        }
    }

    // ------------------------------------------------------------------ battles
    /// Auto-resolve: the strengths, a roll, and losses on both sides (side 0 = the attacker).
    pub fn auto_resolve(&mut self, battle: &PendingBattle) -> BattleResult {
        let (Some(a), Some(d)) = (
            self.army(&battle.attacker).cloned(),
            self.army(&battle.defender).cloned(),
        ) else {
            return BattleResult::default();
        };
        let pa = a.power() * (0.85 + self.rand() * 0.3);
        let pd = d.power() * (0.85 + self.rand() * 0.3) * 1.1;
        let (pw, pdw) = (pa.powf(1.4), pd.powf(1.4));
        let attacker_wins = self.rand() < pw / (pw + pdw);
        let ratio = if attacker_wins { pd / pa } else { pa / pd };
        let win_loss = (0.32 * ratio * (0.7 + self.rand() * 0.6)).min(0.55);
        let lose_loss = 0.45 + self.rand() * 0.3;
        let mut apply = |army: &Army, loss: f32| -> Vec<Survivor> {
            army.units
                .iter()
                .enumerate()
                .map(|(k, u)| {
                    let t = unit(&u.ty);
                    if t.role == Role::Hero {
                        return Survivor {
                            id: format!("u{k}"),
                            ty: u.ty.clone(),
                            men0: u.men,
                            men: 1,
                            kills: 0,
                        };
                    }
                    let kk = (loss * (0.6 + self.rand() * 0.8)).min(1.0);
                    Survivor {
                        id: format!("u{k}"),
                        ty: u.ty.clone(),
                        men0: u.men,
                        men: (u.men as f32 * (1.0 - kk)).round().max(0.0) as u32,
                        kills: 0,
                    }
                })
                .collect()
        };
        let player = apply(&a, if attacker_wins { win_loss } else { lose_loss });
        let enemy = apply(&d, if attacker_wins { lose_loss } else { win_loss });
        BattleResult {
            winner: if attacker_wins { 0 } else { 1 },
            player,
            enemy,
            auto: true,
            label: if attacker_wins {
                "Victoria".into()
            } else {
                "Derrota".into()
            },
            ..Default::default()
        }
    }

    /// Applies a battle's result. `res.player` are the survivors of the result's side 0;
    /// `side_of_attacker` says which side the attacker was in that result; `scale` undoes the
    /// battle's unit size.
    pub fn apply_battle(
        &mut self,
        battle: &PendingBattle,
        res: &BattleResult,
        side_of_attacker: u8,
        scale: f32,
    ) {
        let (a_res, d_res) = if side_of_attacker == 0 {
            (&res.player, &res.enemy)
        } else {
            (&res.enemy, &res.player)
        };
        let attacker_won = (res.winner == 0) == (side_of_attacker == 0);
        let survivors = |army: &mut Army, list: &[Survivor]| {
            let units: Vec<Unit> = army
                .units
                .iter()
                .enumerate()
                .map(|(k, u)| {
                    let r = list
                        .iter()
                        .find(|x| x.id == format!("u{k}"))
                        .or_else(|| list.get(k));
                    let hero = unit(&u.ty).role == Role::Hero;
                    let mut men = match r {
                        Some(r) if hero => r.men,
                        Some(r) => {
                            (r.men as f32 / if res.auto { 1.0 } else { scale }).round() as u32
                        }
                        None => u.men,
                    };
                    if u.ty == "reina" {
                        men = 1; // she withdraws wounded; she never falls
                    }
                    Unit {
                        men: men.min(u.max),
                        ..u.clone()
                    }
                })
                .filter(|u| {
                    if unit(&u.ty).role == Role::Hero {
                        u.men > 0
                    } else {
                        u.men as f32 >= (u.max as f32 * 0.08).max(4.0)
                    }
                })
                .collect();
            army.units = units;
        };
        if let Some(a) = self.army_mut(&battle.attacker) {
            survivors(a, a_res);
        }
        if let Some(d) = self.army_mut(&battle.defender) {
            survivors(d, d_res);
        }
        // Varrak, awake: he leaves his pack and goes to his Queen
        if res.varrak_awake {
            if let Some(pack) = self
                .armies
                .iter_mut()
                .find(|x| x.faction == Faction::Rebeldes && x.has("varrak"))
            {
                pack.units.retain(|u| u.ty != "varrak");
            }
            let qi = self
                .armies
                .iter()
                .position(|x| x.faction == Faction::Reina && x.has("reina"))
                .or_else(|| self.armies.iter().position(|x| x.faction == Faction::Reina));
            if let Some(qi) = qi {
                if self.armies[qi].units.len() < MAX_UNITS && !self.armies[qi].has("varrak_reina") {
                    self.armies[qi]
                        .units
                        .push(new_unit("varrak_reina", Some(1)));
                }
            }
            self.varrak_awake = true;
            self.note(
                "«Majestad. Ha tardado usted.» Varrak despierta y vuelve a su silla del Consejo.",
                LogKind::Spark,
            );
        }
        if res.queen_down {
            self.purse_mut(Faction::Reina).wounded = 2;
            self.note(
                "La Reina se retira herida: la chispa crece más despacio dos turnos.",
                LogKind::Bad,
            );
        }
        let (winner_id, loser_id) = if attacker_won {
            (battle.attacker.clone(), battle.defender.clone())
        } else {
            (battle.defender.clone(), battle.attacker.clone())
        };
        let where_ = province(&battle.prov).name;
        let (wf, lf) = if attacker_won {
            (battle.af, battle.df)
        } else {
            (battle.df, battle.af)
        };
        let wn = self
            .army(&winner_id)
            .map_or(wf.info().short.to_owned(), |a| a.name.clone());
        let ln = self
            .army(&loser_id)
            .map_or(lf.info().short.to_owned(), |a| a.name.clone());
        let loser_left = self.army(&loser_id).is_some_and(|a| !a.units.is_empty());
        let ws = wf.info().short;
        let line = match (attacker_won, loser_left) {
            (true, true) => {
                format!("Batalla de {where_}: {wn} ({ws}) vence al atacar; {ln} se retira.")
            }
            (true, false) => {
                format!("Batalla de {where_}: {wn} ({ws}) vence al atacar; {ln} queda deshecho.")
            }
            (false, true) => format!("Batalla de {where_}: {wn} ({ws}) resiste; {ln} se retira."),
            (false, false) => {
                format!("Batalla de {where_}: {wn} ({ws}) resiste; {ln} queda deshecho.")
            }
        };
        let kind = if wf == Faction::Reina {
            LogKind::Good
        } else if lf == Faction::Reina {
            LogKind::Bad
        } else {
            LogKind::Info
        };
        self.note(line, kind);
        // the loser falls back (or is destroyed)
        if let Some(l) = self.army(&loser_id) {
            if l.units.is_empty() || l.garrison {
                self.armies.retain(|x| x.id != loser_id);
            } else {
                self.retreat(&loser_id);
            }
        }
        if self.army(&winner_id).is_some_and(|w| w.units.is_empty()) {
            self.armies.retain(|x| x.id != winner_id);
        }
        // the attacker who wins walks in (and takes the place if nobody else defends it)
        if attacker_won {
            if let Some(a) = self.army(&battle.attacker) {
                if !a.units.is_empty() {
                    let af = a.faction;
                    let others = self
                        .enemies_at(&battle.prov, af)
                        .any(|x| !x.units.is_empty());
                    if !others {
                        self.army_mut(&battle.attacker).unwrap().at = battle.prov.clone();
                        if self.owner(&battle.prov) != af {
                            self.capture(&battle.prov, af);
                        }
                    }
                }
            }
        }
        // an attack that failed ends its march (it would only throw itself at them again)
        if !attacker_won {
            if let Some(a) = self.army_mut(&battle.attacker) {
                a.march = None;
            }
        }
        self.pending = None;
        self.check_end();
    }

    fn retreat(&mut self, army_id: &str) {
        let Some(army) = self.army(army_id) else {
            return;
        };
        let (at, faction, queen) = (army.at.clone(), army.faction, army.has("reina"));
        let opts: Vec<&str> = adjacent(&at)
            .iter()
            .copied()
            .filter(|p| self.enemies_at(p, faction).next().is_none())
            .collect();
        let to = opts
            .iter()
            .copied()
            .find(|p| self.owner(p) == faction)
            .or(opts.first().copied());
        if to.is_none() && queen && self.owner("castillo") == Faction::Reina && at != "castillo" {
            let a = self.army_mut(army_id).unwrap();
            a.units
                .retain(|u| u.ty == "reina" || u.ty == "varrak_reina");
            a.at = "castillo".into();
            a.mp = 0;
            self.note(
                "La Reina, rodeada, se abre paso con la chispa y vuelve al castillo.",
                LogKind::Bad,
            );
            return;
        }
        let Some(to) = to else {
            let name = army.name.clone();
            self.armies.retain(|x| x.id != army_id);
            self.note(
                format!("{name} no tiene a dónde huir y se dispersa."),
                if faction == Faction::Reina {
                    LogKind::Bad
                } else {
                    LogKind::Good
                },
            );
            return;
        };
        let a = self.army_mut(army_id).unwrap();
        a.at = to.to_owned();
        a.mp = 0;
    }

    /// The player's army steps back from a battle the enemy started (10 % lost on the way).
    pub fn retreat_from(&mut self, battle: &PendingBattle, mine: &str) -> bool {
        let Some(a) = self.army(mine) else {
            return false;
        };
        let from = a.at.clone();
        let to = adjacent(&from).iter().copied().find(|p| {
            self.owner(p) == Faction::Reina
                && !self
                    .armies
                    .iter()
                    .any(|x| x.at == *p && x.faction != Faction::Reina)
        });
        if let Some(to) = to {
            let a = self.army_mut(mine).unwrap();
            for u in &mut a.units {
                if unit(&u.ty).role != Role::Hero {
                    u.men = ((u.men as f32 * 0.9).round() as u32).max(1);
                }
            }
            a.at = to.to_owned();
            a.mp = 0;
            let name = a.name.clone();
            self.note(
                format!(
                    "{} se retira de {} a {}.",
                    name,
                    province(&from).name,
                    province(to).name
                ),
                LogKind::Bad,
            );
            if let Some(att) = self.army(&battle.attacker) {
                let af = att.faction;
                if !self
                    .armies
                    .iter()
                    .any(|x| x.at == battle.prov && x.faction != af)
                {
                    self.army_mut(&battle.attacker).unwrap().at = battle.prov.clone();
                    if self.owner(&battle.prov) != af {
                        self.capture(&battle.prov, af);
                    }
                }
            }
        }
        self.pending = None;
        to.is_some()
    }

    // --------------------------------------------------------------- turn cycle
    /// Ends the player's turn: plans the AI factions' moves into the queue.
    pub fn end_player_turn(&mut self) {
        self.phase = Phase::Ai;
        self.queue.clear();
        for faction in [Faction::Rebeldes, Faction::Alianza] {
            self.plan_faction(faction);
        }
    }

    fn plan_faction(&mut self, faction: Faction) {
        let ids: Vec<String> = self
            .armies
            .iter()
            .filter(|a| a.faction == faction && !a.garrison && !a.units.is_empty())
            .map(|a| a.id.clone())
            .collect();
        for id in ids {
            self.army_mut(&id).unwrap().mp = MP;
            let at = self.army(&id).unwrap().at.clone();
            let mut options: Vec<(Option<&'static str>, f32)> = adjacent(&at)
                .iter()
                .map(|&p| (Some(p), self.score_move(&id, p)))
                .collect();
            options.push((None, self.score_stay(&id)));
            options.sort_by(|x, y| y.1.total_cmp(&x.1));
            if let Some(p) = options[0].0 {
                self.queue.push(QueuedMove {
                    army: id.clone(),
                    to: p.to_owned(),
                });
            }
        }
    }

    /// What an AI army thinks of stepping into p (higher is better).
    fn score_move(&mut self, id: &str, p: &str) -> f32 {
        let a = self.army(id).unwrap().clone();
        let mine = a.power();
        let theirs: f32 = self.enemies_at(p, a.faction).map(|b| b.power()).sum();
        let foes = self.enemies_at(p, a.faction).next().is_some();
        let own = self.owner(p);
        let mut score = if foes {
            if mine > theirs * 1.15 {
                40.0 + mine / theirs * 5.0
            } else {
                -40.0
            }
        } else if own != a.faction {
            let def: f32 = self
                .defenders(p)
                .iter()
                .map(|u| unit_power(unit(&u.ty), u.men))
                .sum();
            if mine > def * 1.2 {
                30.0 + province(p).gold / 20.0
            } else {
                -30.0
            }
        } else {
            -2.0
        };
        // where each faction wants to go
        if a.faction == Faction::Alianza {
            score += 12.0 - hops(p, "castillo") as f32 * 3.0;
        }
        if a.faction == Faction::Rebeldes {
            score += if a.has("varrak") {
                10.0 - hops(p, "villa") as f32 * 3.0
            } else {
                6.0 - hops(p, "bosque") as f32 * 2.0
            };
        }
        // never leave the own capital empty when an enemy is next to it
        for cap in ["capital", "penascal"] {
            if a.at == cap
                && self.owner(cap) == a.faction
                && self.armies_at(cap, Some(a.faction)).count() == 1
            {
                let threat = adjacent(cap)
                    .iter()
                    .any(|q| self.enemies_at(q, a.faction).next().is_some());
                if threat {
                    score -= 50.0;
                }
            }
        }
        score + self.rand() * 4.0
    }

    fn score_stay(&self, id: &str) -> f32 {
        let a = self.army(id).unwrap();
        if a.faction == Faction::Alianza && self.turn < 3 {
            25.0
        } else {
            0.0
        }
    }

    /// Runs the AI queue until it needs the player (a battle against the Queen) or ends. Returns
    /// the steps taken (for the map to animate) and, if any, the battle that waits.
    pub fn run_queue(&mut self) -> (Vec<Step>, Option<PendingBattle>) {
        let mut steps = Vec::new();
        while !self.queue.is_empty() {
            let q = self.queue.remove(0);
            let Some(a) = self.army(&q.army) else {
                continue;
            };
            if a.units.is_empty() || a.mp == 0 || !adjacent(&a.at).contains(&q.to.as_str()) {
                continue;
            }
            let from = a.at.clone();
            let r = self.move_army(&q.army, &q.to);
            if r == MoveResult::Blocked {
                continue;
            }
            steps.push(Step {
                army: q.army.clone(),
                from,
                to: q.to.clone(),
                battle: matches!(r, MoveResult::Battle(_)),
            });
            if let MoveResult::Battle(b) = r {
                if b.df == Faction::Reina || b.af == Faction::Reina {
                    return (steps, Some(b));
                }
                // the AI factions fight each other by auto-resolve
                let res = self.auto_resolve(&b);
                self.apply_battle(&b, &res, 0, 1.0);
            }
        }
        (steps, None)
    }

    /// After the AI's moves: income, upkeep, food, the spark, recruiting, reinforcements.
    pub fn start_turn(&mut self) {
        self.turn += 1;
        self.phase = Phase::Player;
        self.rng = None;
        // the people who rose to defend their place go back to it
        self.armies
            .retain(|a| !(a.garrison && a.faction == Faction::Reina && a.id.starts_with("m-")));
        for faction in [Faction::Reina, Faction::Rebeldes, Faction::Alianza] {
            let inc = self.income(faction);
            self.purse_mut(faction).gold += inc.net_gold;
            if faction == Faction::Reina {
                {
                    let f = self.purse_mut(Faction::Reina);
                    f.food += inc.net_food;
                    f.spark += inc.spark;
                }
                let drain = self.spark_drain();
                self.purse_mut(Faction::Reina).spark -= drain.len() as i32;
                if self.purse(Faction::Reina).spark < 0 {
                    self.purse_mut(Faction::Reina).spark = 0;
                    for id in &drain {
                        let name = if let Some(a) = self.army_mut(id) {
                            for u in &mut a.units {
                                if unit(&u.ty).spark {
                                    u.men = ((u.men as f32 * 0.88).round() as u32).max(1);
                                }
                            }
                            a.name.clone()
                        } else {
                            continue;
                        };
                        self.note(
                            format!("{name} está demasiado lejos de la Reina: algunos se apagan y se van al bosque."),
                            LogKind::Bad,
                        );
                    }
                }
                if self.purse(Faction::Reina).food < 0 {
                    for a in self
                        .armies
                        .iter_mut()
                        .filter(|x| x.faction == Faction::Reina)
                    {
                        for u in &mut a.units {
                            if unit(&u.ty).role != Role::Hero {
                                u.men = ((u.men as f32 * 0.94).round() as u32).max(1);
                            }
                        }
                    }
                    self.note(
                        "Hambre en el reino: las raciones no llegan y los regimientos menguan.",
                        LogKind::Bad,
                    );
                    self.purse_mut(Faction::Reina).food = 0;
                }
                let f = self.purse_mut(Faction::Reina);
                f.wounded = f.wounded.saturating_sub(1);
            }
            let f = self.purse_mut(faction);
            if f.gold < 0 {
                f.gold = 0;
            }
        }
        // regiments refill in their own lands
        for k in 0..self.armies.len() {
            let home = {
                let a = &self.armies[k];
                let st = self.prov(&a.at);
                st.owner == a.faction && st.lit
            };
            let a = &mut self.armies[k];
            for u in &mut a.units {
                if home && unit(&u.ty).role != Role::Hero {
                    u.men = u
                        .max
                        .min((u.men as f32 + u.max as f32 * 0.12).round() as u32);
                }
            }
            a.mp = MP;
        }
        // the AI factions raise troops where they have armies
        for faction in [Faction::Rebeldes, Faction::Alianza] {
            self.ai_recruit(faction);
        }
        // the Alliance sends a new column every fifth turn
        if self.turn % 5 == 0 && self.owner("capital") == Faction::Alianza {
            let n = self
                .armies
                .iter()
                .filter(|a| a.faction == Faction::Alianza && !a.garrison)
                .count();
            if n < 4
                && self
                    .enemies_at("capital", Faction::Alianza)
                    .next()
                    .is_none()
            {
                let id = format!("l-{}", self.next_id);
                self.next_id += 1;
                self.armies.push(Army {
                    id,
                    faction: Faction::Alianza,
                    name: "Columna del Alba".into(),
                    at: "capital".into(),
                    mp: MP,
                    garrison: false,
                    units: [
                        "lanceros",
                        "lanceros",
                        "ballesteros",
                        "caballeros",
                        "sacerdotes",
                    ]
                    .iter()
                    .map(|t| new_unit(t, None))
                    .collect(),
                    march: None,
                });
                self.note(
                    "Una nueva columna de la Alianza sale de la capital caída.",
                    LogKind::Bad,
                );
            }
        }
        // the dimmed gather round Varrak's village
        if self.turn % 4 == 0 && self.owner("penascal") == Faction::Rebeldes {
            if let Some(a) = self
                .armies
                .iter_mut()
                .find(|a| a.at == "penascal" && a.faction == Faction::Rebeldes)
            {
                if a.units.len() < MAX_UNITS {
                    a.units.push(new_unit("apagados", None));
                }
            }
        }
        self.check_end();
    }

    fn ai_recruit(&mut self, faction: Faction) {
        for k in 0..self.armies.len() {
            let (at, n_units, garrison, af) = {
                let a = &self.armies[k];
                (a.at.clone(), a.units.len(), a.garrison, a.faction)
            };
            if af != faction || garrison || self.owner(&at) != faction {
                continue;
            }
            let list = province(&at).recruits(faction);
            if list.is_empty() || n_units >= MAX_UNITS {
                continue;
            }
            let ty = list[((self.rand() * list.len() as f32) as usize).min(list.len() - 1)];
            let cost = unit(ty).cost.gold;
            if self.purse(faction).gold > cost + 200 {
                self.purse_mut(faction).gold -= cost;
                self.armies[k].units.push(new_unit(ty, None));
            }
        }
    }

    /// Provinces the Queen's people can see (hers and next to hers, and round her armies).
    pub fn visible(&self) -> Vec<bool> {
        let mut vis = vec![false; PROVINCES.len()];
        let mut see = |id: &str| vis[crate::data::province_index(id)] = true;
        for (k, p) in PROVINCES.iter().enumerate() {
            if self.provinces[k].owner != Faction::Reina {
                continue;
            }
            see(p.id);
            for q in adjacent(p.id) {
                see(q);
            }
        }
        for a in self.armies.iter().filter(|a| a.faction == Faction::Reina) {
            see(&a.at);
            for q in adjacent(&a.at) {
                see(q);
            }
        }
        vis
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Income {
    pub gold: i32,
    pub food: i32,
    pub spark: i32,
    pub upkeep: i32,
    pub eat: i32,
    pub net_gold: i32,
    pub net_food: i32,
}

#[derive(Clone, Debug)]
pub struct RecruitOption {
    pub ty: &'static str,
    pub ok: bool,
    pub why: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MoveResult {
    Move,
    Capture,
    Battle(PendingBattle),
    Blocked,
}

impl PartialEq for PendingBattle {
    fn eq(&self, other: &Self) -> bool {
        self.attacker == other.attacker
            && self.defender == other.defender
            && self.prov == other.prov
    }
}

#[derive(Clone, Debug)]
pub struct Step {
    pub army: String,
    pub from: String,
    pub to: String,
    pub battle: bool,
}

/// Province graph distance (for the spark's reach).
pub fn hops(from: &str, to: &str) -> u32 {
    if from == to {
        return 0;
    }
    let mut seen = vec![from];
    let mut frontier = vec![from];
    let mut d = 0;
    while !frontier.is_empty() {
        d += 1;
        let mut next = Vec::new();
        for p in frontier {
            for &q in adjacent(p) {
                if q == to {
                    return d;
                }
                if !seen.contains(&q) {
                    seen.push(q);
                    next.push(q);
                }
            }
        }
        frontier = next;
    }
    99
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_campaign_plays_many_turns_without_breaking() {
        let mut s = Campaign::new(42, false);
        for _ in 0..30 {
            if s.winner.is_some() {
                break;
            }
            s.end_player_turn();
            loop {
                let (_, battle) = s.run_queue();
                match battle {
                    Some(b) => {
                        let res = s.auto_resolve(&b);
                        s.apply_battle(&b, &res, 0, 1.0);
                    }
                    None => break,
                }
            }
            s.start_turn();
        }
        assert!(s.turn > 5);
        // it survives a save and a load
        let json = serde_json::to_string(&s).unwrap();
        let back: Campaign = serde_json::from_str(&json).unwrap();
        assert_eq!(back.turn, s.turn);
        assert_eq!(back.armies.len(), s.armies.len());
    }

    #[test]
    fn the_queen_moves_recruits_and_captures() {
        let mut s = Campaign::new(3, false);
        let q = s.queen_army().unwrap().id.clone();
        assert!(s.reachable(s.army(&q).unwrap()).contains(&"villa"));
        assert_eq!(s.move_army(&q, "villa"), MoveResult::Move);
        assert!(s.recruit("castillo", "orcos", Faction::Reina).is_some());
        // the guard walks east into the Alliance's march: a battle against the fort
        let g = "a-guardia";
        s.army_mut(g).unwrap().at = "puerta".into();
        s.army_mut(g).unwrap().mp = 2;
        assert!(matches!(s.move_army(g, "marca"), MoveResult::Battle(_)));
    }

    #[test]
    fn a_march_finds_its_way_and_counts_its_turns() {
        let s = Campaign::new(5, false);
        let q = s.queen_army().unwrap().clone();
        // castle → forest → the gate: the forest is hers (one move), the gate is the Alliance's
        let way = s.route(&q, "puerta").expect("a way east");
        assert_eq!(way.last(), Some(&"puerta"));
        assert!(way.iter().all(|p| *p != "castillo"));
        for pair in std::iter::once(&q.at.as_str())
            .chain(way.iter())
            .collect::<Vec<_>>()
            .windows(2)
        {
            assert!(adjacent(pair[0]).contains(pair[1]), "{pair:?}");
        }
        let turns = s.march_turns(&q, &way);
        assert_eq!(turns.len(), way.len());
        assert!(turns.windows(2).all(|t| t[0] <= t[1]));
        assert!(s.route(&q, "castillo").is_none(), "already there");
        // the far capital takes more turns than the next valley
        let far = s.route(&q, "capital").unwrap();
        assert!(
            *s.march_turns(&q, &far).last().unwrap()
                > *s.march_turns(&q, &s.route(&q, "villa").unwrap())
                    .last()
                    .unwrap()
        );
    }

    #[test]
    fn the_queens_lit_lands_rise_against_an_attacker_and_unlit_ones_fall() {
        let mut s = Campaign::new(8, false);
        // a rebel band walks into the village, hers and lit, with no army of hers there
        let r = s
            .armies
            .iter()
            .find(|a| a.faction == Faction::Rebeldes && !a.garrison)
            .unwrap()
            .id
            .clone();
        s.armies
            .retain(|a| !(a.faction == Faction::Reina && a.at == "villa"));
        s.army_mut(&r).unwrap().at = "madriguera".into();
        s.army_mut(&r).unwrap().mp = 2;
        assert!(!s.defenders("villa").is_empty());
        match s.move_army(&r, "villa") {
            MoveResult::Battle(b) => {
                assert_eq!(b.df, Faction::Reina);
                let militia = s.army(&b.defender).unwrap();
                assert!(militia.garrison && militia.name.starts_with("Milicia"));
            }
            other => panic!("the village fell without a fight: {other:?}"),
        }
        // the militia goes home when the turn turns
        s.pending = None;
        s.start_turn();
        assert!(!s.armies.iter().any(|a| a.id.starts_with("m-")));
        // without the spark nobody rises
        s.set_owner("villa", Faction::Reina, false);
        assert!(s.defenders("villa").is_empty());
    }

    #[test]
    fn if_the_story_woke_varrak_he_rides_with_her() {
        let s = Campaign::new(1, true);
        assert_eq!(s.owner("penascal"), Faction::Reina);
        assert!(s.queen_army().unwrap().has("varrak_reina"));
        assert!(s.army("r-manada").is_none());
    }
}
