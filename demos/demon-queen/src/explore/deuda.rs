//! Rosalía's loans, on the world's side: the days that pass (`dia_mundo`, for its due date), the
//! door that stays shut to a Queen who would not pay, her errand down the miner's shaft (straight
//! from the book to the shaft with a quota for her wagon; the mine's `encargo.rs` keeps the cage
//! shut until it is met or given up), and the shifts in the house (each a scene of the Queen with a
//! client, `burdel_turno_<n>`, then his minigame, `turno.rs`: his tip comes off the debt). Once a
//! debt is worked off, the same shift and errand are the house's jobs, for pay.
//! The deal itself lives in Rosalía's book (`venue.rs`).
use super::mascot::Cue;
use super::*;

/// The story variable that counts the world's days.
pub(super) const WORLD_DAY: &str = "dia_mundo";
use super::play_scene::turno;

impl App {
    /// The world's day: one more each midnight the clock passes (and each night slept through).
    pub(super) fn world_day(&self) -> i32 {
        self.story.get(WORLD_DAY)
    }

    /// The clock moves on; past midnight a new day begins.
    pub(super) fn advance_world_clock(&mut self, dt: f32) {
        let before = self.environment_cycle.phase();
        self.environment_cycle.advance(dt);
        if self.environment_cycle.phase() < before {
            self.new_world_day();
        }
    }

    /// The clock set forward to `phase` (sleeping): a new day if it went past midnight.
    pub(super) fn set_world_phase(&mut self, phase: f32) {
        let before = self.environment_cycle.phase();
        self.environment_cycle.set_phase(phase);
        if self.environment_cycle.phase() < before {
            self.new_world_day();
        }
    }

    fn new_world_day(&mut self) {
        *self.story.values.entry(WORLD_DAY.to_owned()).or_insert(0) += 1;
        self.venue.today = self.world_day();
        let p = &self.venue.progress;
        if p.debt > 0 && p.debt_way.is_empty() {
            if self.world_day() == p.debt_due {
                self.notice_note(format!(
                    "La deuda con Rosalía vence hoy\nLe debes {} oro. Págalo en el libro de su recepción.",
                    p.debt
                ));
            } else if self.world_day() == p.debt_due + 1 {
                self.notice_note(format!(
                    "La deuda con Rosalía ha vencido\nLe debes {} oro. En su recepción elegirás cómo pagarlo.",
                    p.debt
                ));
                self.float_chiquita_cue(
                    "—Te lo dije. Nunca le pidas oro a una mujer que sonríe tanto.",
                    Cue::Scold,
                );
            }
        }
    }

    /// The door of the house for a Queen who would not pay: shut, unless she pays it here and now.
    /// Whether she may go in.
    pub(super) fn brothel_door_open(&mut self) -> bool {
        if !self.venue.banned() {
            return true;
        }
        let owed = self.venue.progress.debt;
        if self.venue.pay_at_door(&mut self.gold) {
            self.ui_sound(dungeon::sound::Sfx::Coins, 0.8);
            self.notice_note(format!(
                "Deuda saldada\nPagas los {owed} oro en la puerta y Rosalía te deja pasar."
            ));
            self.persist_3d_state(true);
            return true;
        }
        self.float_chiquita_cue(
            "—«Hasta que pague, ni un pie dentro». Lo ha dicho con una sonrisa. Da más miedo que un orco.",
            Cue::Scold,
        );
        self.notice_note(format!(
            "Rosalía no te deja pasar\nLe debes {owed} oro. Vuelve con el oro y se paga en la puerta."
        ));
        false
    }

    /// Paying in the mine: Rosalía's share of a sale (all of it, up to the debt); a note when it
    /// is settled.
    pub(super) fn rosalia_cut(&mut self, earned: u32) -> u32 {
        let cut = self.venue.takes_from_sale(earned);
        if cut > 0 && self.venue.progress.debt == 0 {
            self.notice_note(
                "Deuda saldada\nEl enano le hace llegar a Rosalía lo último. La casa vuelve a abrirte el libro."
                    .to_owned(),
            );
        }
        cut
    }

    /// After Enter in the book: a shift chosen plays its scene; a Queen who would not pay is
    /// seen out. Whether the book was handled here.
    pub(super) fn after_loan_act(&mut self) -> bool {
        // Down the shaft for Rosalía, straight from her book.
        if let Some((quota, pay, debt)) = self.venue.to_mine.take() {
            self.close_shop();
            self.leave_brothel();
            self.enter_dungeon_unchecked();
            self.dungeon.start_errand(quota, pay, debt);
            self.notice_note(if debt {
                format!(
                    "Al pozo, por Rosalía
Pica {quota} oro en mineral para su vagoneta. La jaula no sube hasta cumplirlo."
                )
            } else {
                format!(
                    "Encargo del pozo
Pica {quota} oro en mineral para su vagoneta y cobras {pay}."
                )
            });
            self.float_chiquita_cue(
                "—¿Picar piedra? ¿Tú? …Vale. Pero yo no cargo nada. Tengo las manos de porcelana.",
                Cue::Scold,
            );
            self.persist_3d_state(true);
            return true;
        }
        // A client chosen on his card: his scene and his minigame. In debt, his pay comes off it;
        // otherwise it is hers, and his tip on top.
        if let Some(scene) = self.venue.client_shift.take() {
            let card = venue::ROSTER.iter().find(|c| c.scene == scene);
            let (id, name) = card.map_or(("", "Un cliente"), |c| (c.id, c.name));
            *self.venue.progress.served.entry(id.to_owned()).or_default() += 1;
            let note = if self.venue.progress.debt > 0 {
                let paid = self.venue.shift_done();
                match self.venue.progress.debt {
                    0 => format!("{name}\n−{paid} oro de deuda. Deuda saldada."),
                    left => format!("{name}\n−{paid} oro de deuda. Quedan {left}."),
                }
            } else {
                self.venue.progress.shifts_worked += 1;
                self.venue.progress.jobs_shifts += 1;
                self.gold = self.gold.saturating_add(venue::SHIFT_PAY);
                format!(
                    "{name}\n+{} oro. Y su propina, si le gusta.",
                    venue::SHIFT_PAY
                )
            };
            self.close_shop();
            self.persist_3d_state(true);
            self.notice_note(note);
            self.start_talk(None, Some(&format!("burdel_turno_{scene}")));
            return true;
        }
        if self.venue.shift {
            self.venue.shift = false;
            // The clients take turns (`turno::ROTATION`, the guion's scene numbers).
            let n =
                turno::ROTATION[self.venue.progress.shifts_worked as usize % turno::ROTATION.len()];
            let paid = self.venue.shift_done();
            let left = self.venue.progress.debt;
            self.close_shop();
            self.persist_3d_state(true);
            self.notice_note(if left == 0 {
                format!("Turno en la casa\n−{paid} oro de deuda. Deuda saldada.")
            } else {
                format!("Turno en la casa\n−{paid} oro de deuda. Quedan {left}.")
            });
            self.start_talk(None, Some(&format!("burdel_turno_{n}")));
            return true;
        }
        if self.venue.expelled {
            self.venue.expelled = false;
            self.close_shop();
            self.float_chiquita_cue(
                "—Bien hecho. Dignidad ante todo. …Ahora a ver dónde pasamos las noches.",
                Cue::Cheer,
            );
            self.notice_note(format!(
                "Vetada en el burdel\nNo vuelves a entrar hasta que pagues los {} oro (se paga en la puerta).",
                self.venue.progress.debt
            ));
            self.leave_brothel();
            self.persist_3d_state(true);
            return true;
        }
        false
    }

    /// Each frame in the mine: a debt being worked off in the shaft gets its errand back if she is
    /// down there without one (after a reload); an errand that ended is settled with Rosalía.
    pub(super) fn settle_errand(&mut self) {
        let p = &self.venue.progress;
        if self.dungeon.pozo.is_some()
            && self.dungeon.errand.is_none()
            && p.debt > 0
            && p.debt_way == venue::WAY_MINE
        {
            let debt = p.debt;
            self.dungeon.start_errand(debt, 0, true);
        }
        let Some(met) = self.dungeon.errand.as_ref().and_then(|e| e.outcome) else {
            return;
        };
        let Some(errand) = self.dungeon.errand.take() else {
            return;
        };
        let note =
            self.venue
                .errand_done(met, errand.pay, errand.debt, errand.done, &mut self.gold);
        if met {
            self.ui_sound(dungeon::sound::Sfx::Coins, 0.8);
            self.float_chiquita_cue(
                if errand.debt {
                    "—Deuda pagada a golpe de pico. Si tu corte se entera… no se entera. Yo no digo nada."
                } else {
                    "—Oro honrado. Qué raro se siente. Como cosquillas en el orgullo."
                },
                Cue::Cheer,
            );
        } else if errand.debt {
            self.float_chiquita_cue(
                "—Vetadas. Otra vez. A este paso nos van a vetar hasta en el castillo.",
                Cue::Scold,
            );
        }
        self.notice_note(note);
        self.persist_3d_state(true);
    }

    /// A shift's minigame closed: the tip he left comes off the debt (or into her purse once it is
    /// paid). The first time a shift goes all the way, the Queen loses her virginity to him.
    pub(super) fn after_turno(&mut self, client: usize, complete: bool, tip: u32) {
        let c = &turno::CLIENTS[client.min(turno::CLIENTS.len() - 1)];
        if complete
            && matches!(c.id, "gorm" | "varg")
            && self.story.get(super::ero::VIRGINIDAD_PERDIDA) <= 0
        {
            self.story
                .values
                .insert(super::ero::VIRGINIDAD_PERDIDA.to_owned(), 1);
        }
        if tip == 0 {
            return;
        }
        let off = self.venue.take_tip(tip);
        let rest = tip - off;
        self.gold = self.gold.saturating_add(rest);
        self.notice_note(match (off, rest, self.venue.progress.debt) {
            (0, r, _) => format!(
                "Propina de {}
+{r} oro.",
                c.name
            ),
            (o, 0, 0) => format!(
                "Propina de {}
−{o} oro de deuda. Deuda saldada.",
                c.name
            ),
            (o, 0, left) => format!(
                "Propina de {}
−{o} oro de deuda. Quedan {left}.",
                c.name
            ),
            (o, r, _) => format!(
                "Propina de {}
−{o} oro de deuda (saldada) y +{r} oro.",
                c.name
            ),
        });
    }
}
