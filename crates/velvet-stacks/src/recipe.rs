//! Recipes: a stack whose cards exactly fill a recipe's ingredients starts that recipe's job.
//!
//! "Exactly" means every card of the stack is used by one ingredient and every ingredient gets all
//! the cards it asks for. Two selectors may both fit the same card, so matching is an assignment
//! problem, solved with augmenting paths (Kuhn's algorithm) rather than by taking cards greedily.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::def::{check_selector, CardDef, Selector};
use crate::error::StacksError;

/// Some cards a recipe needs: `count` cards that match `select`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    /// Which cards qualify.
    pub select: Selector,
    /// How many of them the recipe needs. At least 1.
    #[serde(default = "one")]
    pub count: u32,
    /// Whether those cards are used up when the job finishes. Workers and zones usually are not.
    #[serde(default = "yes")]
    pub consume: bool,
}

/// Something a finished job creates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    /// The card definition to create.
    pub card: String,
    /// How many copies to create. At least 1.
    #[serde(default = "one")]
    pub count: u32,
    /// The chance, in `0..=1`, that this output appears. Chances of 1 never draw from the random
    /// generator; any lower chance draws once per copy.
    #[serde(default = "always")]
    pub chance: f32,
}

/// A recipe: the cards it needs, how long it takes, and what it makes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Unique id. The last tie-break between recipes is their id order.
    pub id: String,
    /// The cards a stack must hold, exactly.
    pub ingredients: Vec<Ingredient>,
    /// Seconds the job takes. Zero finishes on the next tick.
    pub seconds: f32,
    /// What the job creates. May be empty: the ingredients are then simply used up.
    #[serde(default)]
    pub outputs: Vec<Output>,
    /// Higher wins when several recipes match the same stack.
    #[serde(default)]
    pub priority: i32,
}

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

fn always() -> f32 {
    1.0
}

impl Recipe {
    /// How many cards the recipe needs in total: the sum of its ingredient counts.
    pub fn ingredient_cards(&self) -> u64 {
        self.ingredients.iter().map(|i| u64::from(i.count)).sum()
    }

    /// The order recipes are tried in: higher priority first, then more ingredient cards, then the
    /// id in byte order.
    pub fn cmp_precedence(a: &Recipe, b: &Recipe) -> Ordering {
        b.priority
            .cmp(&a.priority)
            .then_with(|| b.ingredient_cards().cmp(&a.ingredient_cards()))
            .then_with(|| a.id.cmp(&b.id))
    }

    /// Assigns each card of a stack to the ingredient it fills, if the cards match exactly.
    ///
    /// `defs` are the stack's cards from bottom to top. The result has one entry per card: the
    /// index of the ingredient that card fills. `None` when the cards do not match exactly.
    pub fn assign(&self, defs: &[&CardDef]) -> Option<Vec<usize>> {
        if defs.len() as u64 != self.ingredient_cards() {
            return None;
        }
        // One unit per card the recipe asks for; `owner[unit]` is the ingredient the unit belongs to.
        let owner: Vec<usize> = self
            .ingredients
            .iter()
            .enumerate()
            .flat_map(|(index, ingredient)| std::iter::repeat_n(index, ingredient.count as usize))
            .collect();
        let fits: Vec<Vec<bool>> = defs
            .iter()
            .map(|def| {
                owner
                    .iter()
                    .map(|&index| {
                        self.ingredients
                            .get(index)
                            .is_some_and(|ingredient| ingredient.select.matches(def))
                    })
                    .collect()
            })
            .collect();
        let mut card_of_unit: Vec<Option<usize>> = vec![None; owner.len()];
        for card in 0..defs.len() {
            let mut tried = vec![false; owner.len()];
            if !place(card, &fits, &mut tried, &mut card_of_unit) {
                return None;
            }
        }
        // Every card holds one unit and there are as many units as cards, so every unit is filled.
        let mut result = vec![0; defs.len()];
        for (unit, card) in card_of_unit.iter().enumerate() {
            if let (Some(card), Some(&index)) = (card, owner.get(unit)) {
                result[*card] = index;
            }
        }
        Some(result)
    }

    /// Whether the stack's cards match this recipe exactly. See [`Recipe::assign`].
    pub fn matches(&self, defs: &[&CardDef]) -> bool {
        self.assign(defs).is_some()
    }

    /// Checks the recipe against the ids the catalog knows. Used by [`crate::Catalog::new`].
    pub(crate) fn validate(&self, known: &impl Fn(&str) -> bool) -> Result<(), StacksError> {
        if self.ingredients.is_empty() {
            return Err(StacksError::NoIngredients {
                recipe: self.id.clone(),
            });
        }
        if !(self.seconds.is_finite() && self.seconds >= 0.0) {
            return Err(StacksError::BadSeconds {
                recipe: self.id.clone(),
                seconds: self.seconds,
            });
        }
        let context = format!("an ingredient of recipe `{}`", self.id);
        for ingredient in &self.ingredients {
            if ingredient.count == 0 {
                return Err(StacksError::ZeroCount { context });
            }
            check_selector(&ingredient.select, known, &context)?;
        }
        let context = format!("an output of recipe `{}`", self.id);
        for output in &self.outputs {
            if !known(output.card.as_str()) {
                return Err(StacksError::UnknownCard {
                    id: output.card.clone(),
                    context,
                });
            }
            if output.count == 0 {
                return Err(StacksError::ZeroCount { context });
            }
            if !(output.chance.is_finite() && (0.0..=1.0).contains(&output.chance)) {
                return Err(StacksError::BadChance {
                    recipe: self.id.clone(),
                    chance: output.chance,
                });
            }
        }
        Ok(())
    }
}

/// Tries to give `card` a unit, moving the card that holds a unit it wants to another free unit
/// when needed. `tried` marks the units already explored in this search.
fn place(
    card: usize,
    fits: &[Vec<bool>],
    tried: &mut [bool],
    card_of_unit: &mut [Option<usize>],
) -> bool {
    for (unit, &allowed) in fits[card].iter().enumerate() {
        if !allowed || tried[unit] {
            continue;
        }
        tried[unit] = true;
        let free = match card_of_unit[unit] {
            None => true,
            Some(other) => place(other, fits, tried, card_of_unit),
        };
        if free {
            card_of_unit[unit] = Some(card);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::def::StackRule;

    fn card(id: &str, tags: &[&str]) -> CardDef {
        CardDef {
            id: id.to_string(),
            name: id.to_string(),
            kind: "resource".to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            stack: StackRule::default(),
            value: 0,
        }
    }

    fn by_id(id: &str) -> Selector {
        Selector {
            id: Some(id.to_string()),
            ..Selector::default()
        }
    }

    fn by_tag(tag: &str) -> Selector {
        Selector {
            tag: Some(tag.to_string()),
            ..Selector::default()
        }
    }

    fn ingredient(select: Selector, count: u32) -> Ingredient {
        Ingredient {
            select,
            count,
            consume: true,
        }
    }

    fn recipe(id: &str, ingredients: Vec<Ingredient>, priority: i32) -> Recipe {
        Recipe {
            id: id.to_string(),
            ingredients,
            seconds: 1.0,
            outputs: Vec::new(),
            priority,
        }
    }

    #[test]
    fn matches_exact_counts_only() {
        let house = recipe(
            "house",
            vec![ingredient(by_id("wood"), 2), ingredient(by_id("stone"), 1)],
            0,
        );
        let wood = card("wood", &[]);
        let stone = card("stone", &[]);
        assert!(house.matches(&[&wood, &wood, &stone]));
        assert!(!house.matches(&[&wood, &stone]), "one wood is missing");
        assert!(
            !house.matches(&[&wood, &wood, &stone, &stone]),
            "extra stone"
        );
        assert!(!house.matches(&[&wood, &wood, &wood]), "no stone");
    }

    #[test]
    fn backtracks_when_selectors_overlap() {
        // A plank is also wood. Taking cards greedily would give the plank to the first ingredient
        // and leave the wood with nothing; the assignment must move the plank across.
        let recipe = recipe(
            "fence",
            vec![ingredient(by_tag("wood"), 1), ingredient(by_id("plank"), 1)],
            0,
        );
        let plank = card("plank", &["wood"]);
        let wood = card("wood", &["wood"]);
        assert_eq!(recipe.assign(&[&plank, &wood]), Some(vec![1, 0]));
    }

    #[test]
    fn reports_which_ingredient_each_card_fills() {
        let worker_at_forest = recipe(
            "gather",
            vec![
                ingredient(by_tag("gather"), 1),
                ingredient(by_id("forest"), 1),
            ],
            0,
        );
        let worker = card("worker", &["gather"]);
        let forest = card("forest", &[]);
        assert_eq!(
            worker_at_forest.assign(&[&forest, &worker]),
            Some(vec![1, 0])
        );
        assert_eq!(
            worker_at_forest.assign(&[&worker, &forest]),
            Some(vec![0, 1])
        );
    }

    #[test]
    fn orders_by_priority_then_ingredient_cards_then_id() {
        let low = recipe("a", vec![ingredient(by_id("x"), 1)], 0);
        let high = recipe("z", vec![ingredient(by_id("x"), 1)], 3);
        let more_cards = recipe(
            "b",
            vec![ingredient(by_id("x"), 2), ingredient(by_id("y"), 1)],
            0,
        );
        let mut all = [low.clone(), high.clone(), more_cards.clone()];
        all.sort_by(Recipe::cmp_precedence);
        let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["z", "b", "a"]);
        assert_eq!(Recipe::cmp_precedence(&low, &low), Ordering::Equal);
    }

    #[test]
    fn validation_names_the_bad_field() {
        let known = |id: &str| id == "wood";
        let no_inputs = recipe("r", Vec::new(), 0);
        assert!(matches!(
            no_inputs.validate(&known),
            Err(StacksError::NoIngredients { .. })
        ));
        let zero = recipe("r", vec![ingredient(by_id("wood"), 0)], 0);
        assert!(matches!(
            zero.validate(&known),
            Err(StacksError::ZeroCount { .. })
        ));
        let mut slow = recipe("r", vec![ingredient(by_id("wood"), 1)], 0);
        slow.seconds = f32::NAN;
        assert!(matches!(
            slow.validate(&known),
            Err(StacksError::BadSeconds { .. })
        ));
        let mut odd = recipe("r", vec![ingredient(by_id("wood"), 1)], 0);
        odd.outputs = vec![Output {
            card: "wood".into(),
            count: 1,
            chance: 1.5,
        }];
        assert!(matches!(
            odd.validate(&known),
            Err(StacksError::BadChance { .. })
        ));
    }
}
