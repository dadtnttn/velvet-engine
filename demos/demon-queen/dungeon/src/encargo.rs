//! Rosalía's errand down the miner's shaft (the game's `venue.rs`: a debt to work off, or a job of
//! the house): a quota of what the shaft gives, in gold at the village's price (ore and fossils at
//! theirs, each block of plain rock at one, for her builders). While it is short, all she digs goes
//! up to Rosalía's wagon by the cage instead of into her bag, and the cage will not take her out;
//! C twice gives the errand up (a debt's: the house bans her until it is paid).

/// Each block of plain rock, to Rosalía's builders.
pub const ROCK_VALUE: u32 = 1;
/// The second C must come this soon after the first to give the errand up.
pub const CONFIRM_SECONDS: f32 = 3.0;

#[derive(Clone, Debug, Default)]
pub struct Errand {
    /// What Rosalía wants, in gold at the village's price.
    pub quota: u32,
    /// What her wagon holds so far.
    pub done: u32,
    /// Blocks broken and pieces of ore sent up while on it.
    pub blocks: u32,
    pub pieces: u32,
    /// What the house pays once it is met (a job), or 0 (a debt: it is written off instead).
    pub pay: u32,
    /// Working off a debt (giving it up shuts the house to her).
    pub debt: bool,
    /// The mine's clock when it began.
    pub since: f32,
    /// The first C, when (a second one soon after gives it up).
    pub armed: Option<f32>,
    /// How it ended, once she is out of the shaft: met (`true`) or given up.
    pub outcome: Option<bool>,
}

impl Errand {
    pub fn new(quota: u32, pay: u32, debt: bool, now: f32) -> Self {
        Self {
            quota: quota.max(1),
            pay,
            debt,
            since: now,
            ..Default::default()
        }
    }

    /// Rosalía has all she asked for.
    pub fn met(&self) -> bool {
        self.done >= self.quota
    }

    /// How far along, 0..=1.
    pub fn share(&self) -> f32 {
        (self.done as f32 / self.quota.max(1) as f32).min(1.0)
    }

    /// Gold sent up: what it adds, and whether this is what met the quota.
    pub fn add(&mut self, gold: u32) -> bool {
        let before = self.met();
        self.done = self.done.saturating_add(gold);
        !before && self.met()
    }

    /// C: the first arms, a second within [`CONFIRM_SECONDS`] gives it up. Whether it was given up.
    pub fn press_cancel(&mut self, now: f32) -> bool {
        match self.armed {
            Some(at) if now - at <= CONFIRM_SECONDS => {
                self.outcome = Some(false);
                true
            }
            _ => {
                self.armed = Some(now);
                false
            }
        }
    }

    /// Whether the warning of the first C is still up.
    pub fn arming(&self, now: f32) -> bool {
        self.armed.is_some_and(|at| now - at <= CONFIRM_SECONDS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quota_is_met_once_and_cancel_asks_twice() {
        let mut e = Errand::new(10, 0, true, 0.0);
        assert!(!e.add(6));
        assert!(e.add(5), "this one meets it");
        assert!(!e.add(3), "only once");
        assert!(e.met());
        let mut e = Errand::new(10, 0, true, 0.0);
        assert!(!e.press_cancel(1.0));
        assert!(e.arming(2.0));
        assert!(!e.press_cancel(9.0), "too late: armed again");
        assert!(e.press_cancel(10.0));
        assert_eq!(e.outcome, Some(false));
    }
}
