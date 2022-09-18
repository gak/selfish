use crate::{Game, GameCard, PlayerReference, SpaceCard};

/// Information that a fair player can observe about the game.
///
/// * Whose turn it is
/// * The number of cards in each player's hand
/// * The space grid.
pub struct VisibleState {
    pub whose_turn: PlayerReference,
    pub my_hand: Vec<GameCard>,
    pub players: Vec<VisiblePlayer>,
}

pub struct VisiblePlayer {
    pub alive: bool,
    pub hand_size: usize,
    pub space: Vec<SpaceCard>,
}

impl VisibleState {
    pub fn try_from_game(game: &Game) -> miette::Result<Self> {
        let whose_turn = game.whose_turn_reference;
        let players = (0..game.player_count())
            .map(|player_reference| {
                let player = game.player(&PlayerReference(player_reference))?;
                Ok(VisiblePlayer {
                    hand_size: player.hand.len(),
                    space: player.space.clone(),
                    alive: player.alive,
                })
            })
            .collect::<miette::Result<Vec<VisiblePlayer>>>()?;
        let my_hand = game.player(&whose_turn)?.hand.clone();

        Ok(VisibleState {
            whose_turn,
            my_hand,
            players,
        })
    }

    pub fn invalid() -> Self {
        VisibleState {
            whose_turn: PlayerReference(42),
            my_hand: vec![],
            players: Vec::new(),
        }
    }
}
