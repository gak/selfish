use crate::{GameCard, PlayerReference};
use miette::Diagnostic;
use thiserror::Error;

/// An error specific to a player attempting an action incorrectly.
#[derive(Error, Debug, Diagnostic)]
pub enum SelfishError {
    #[error("You can't attack yourself!")]
    CantAttackYourself,

    #[error("Player has no cards!")]
    PlayerHasNoCardsLeft,

    #[error("Player {0:?} does not have enough cards to steal {1}!")]
    PlayerDoesNotHaveEnoughCards(PlayerReference, usize),

    #[error("You don't have a {0:?} card!")]
    PlayerDoesNotHaveThisCard(GameCard),

    #[error("Player {0:?} does not exist!")]
    PlayerDoesNotExist(PlayerReference),

    #[error("Invalid discard count. Expected {expected} but got {actual}.")]
    InvalidDiscardCount { expected: usize, actual: usize },
}
