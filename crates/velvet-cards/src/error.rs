//! Errors for card tools.

use thiserror::Error;

use crate::catalog::CardId;
use crate::zones::ZoneKind;

/// Card tooling errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardError {
    /// Card id missing from catalog.
    #[error("unknown card id `{0}`")]
    UnknownCard(CardId),
    /// Deck failed validation (see violations for detail).
    #[error("deck validation failed ({0} violation(s))")]
    ValidationFailed(usize),
    /// Not enough cards in the source zone.
    #[error("not enough cards in {zone:?}: need {need}, have {have}")]
    NotEnough {
        /// Zone that was short.
        zone: ZoneKind,
        /// Requested count.
        need: usize,
        /// Available count.
        have: usize,
    },
    /// Index out of range in a zone.
    #[error("index {index} out of range in {zone:?} (len {len})")]
    IndexOutOfRange {
        /// Zone.
        zone: ZoneKind,
        /// Index requested.
        index: usize,
        /// Zone length.
        len: usize,
    },
    /// JSON / IO parse failure.
    #[error("parse error: {0}")]
    Parse(String),
    /// IO failure.
    #[error("io error: {0}")]
    Io(String),
}
