//! Errors of the stacks engine: documents that do not load, definitions that do not validate, and
//! board actions the rules refuse.

use thiserror::Error;

/// Everything the stacks engine can refuse.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum StacksError {
    /// The text is not JSON of the shape the document needs.
    #[error("cannot read the document: {0}")]
    Parse(String),
    /// The document's `format`, `kind` or `version` is not one this engine reads.
    #[error("expected a velvet-stacks {expected} document (version 1), found {found}")]
    UnsupportedDocument {
        /// The kind of document the caller asked for.
        expected: String,
        /// What the document says it is.
        found: String,
    },
    /// A card has an empty id.
    #[error("a card id must not be empty")]
    EmptyCardId,
    /// Two cards share an id.
    #[error("duplicate card id `{0}`")]
    DuplicateCard(String),
    /// A stack rule allows a stack of zero cards.
    #[error("card `{card}` has a stack limit of zero")]
    ZeroMax {
        /// The card whose rule is wrong.
        card: String,
    },
    /// An id names a card that is not in the catalog.
    #[error("unknown card id `{id}` in {context}")]
    UnknownCard {
        /// The id that was not found.
        id: String,
        /// Where it was used.
        context: String,
    },
    /// Two recipes share an id.
    #[error("duplicate recipe id `{0}`")]
    DuplicateRecipe(String),
    /// A save names a recipe that is not in the catalog.
    #[error("unknown recipe id `{0}`")]
    UnknownRecipe(String),
    /// A recipe has no ingredients, so it would match every stack.
    #[error("recipe `{recipe}` has no ingredients")]
    NoIngredients {
        /// The recipe.
        recipe: String,
    },
    /// An ingredient or output asks for zero cards.
    #[error("{context} asks for zero cards")]
    ZeroCount {
        /// Where the count sits.
        context: String,
    },
    /// A recipe's duration is negative or not a number.
    #[error("recipe `{recipe}` has a duration of {seconds} seconds")]
    BadSeconds {
        /// The recipe.
        recipe: String,
        /// The value given.
        seconds: f32,
    },
    /// An output's chance is outside `0..=1` or not a number.
    #[error("recipe `{recipe}` has an output chance of {chance}")]
    BadChance {
        /// The recipe.
        recipe: String,
        /// The value given.
        chance: f32,
    },
    /// A board action names a card that is not on the board.
    #[error("no card {0} on the board")]
    NoSuchCard(u64),
    /// A card was dropped onto its own stack.
    #[error("card {0} is already on the stack it was dropped on")]
    SameStack(u64),
    /// The card on top of the target does not accept the dragged card.
    #[error("card {card} may not be placed on card {onto}")]
    DropRefused {
        /// The bottom card of the dragged stack.
        card: u64,
        /// The top card of the target stack.
        onto: u64,
    },
    /// The drop would make the stack taller than the target's rule allows.
    #[error("a stack on card {onto} may hold at most {max} cards, it would hold {height}")]
    StackFull {
        /// The top card of the target stack.
        onto: u64,
        /// The rule's limit.
        max: u32,
        /// The height the stack would reach.
        height: u32,
    },
    /// A save is internally inconsistent.
    #[error("invalid board save: {0}")]
    InvalidSave(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_ids_involved() {
        let refused = StacksError::DropRefused { card: 2, onto: 1 };
        assert_eq!(refused.to_string(), "card 2 may not be placed on card 1");
        let unknown = StacksError::UnknownCard {
            id: "ghost".into(),
            context: "spawn".into(),
        };
        assert_eq!(unknown.to_string(), "unknown card id `ghost` in spawn");
        let full = StacksError::StackFull {
            onto: 3,
            max: 1,
            height: 2,
        };
        assert!(full.to_string().contains("at most 1"));
    }
}
