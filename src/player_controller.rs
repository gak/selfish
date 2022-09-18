use crate::actions::BreatheOrTravel;
use crate::visible_state::VisibleState;
use crate::{Action, GameCard, PlayerReference};
use rand::prelude::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub trait PlayerController {
    /// Give the player only the information that they would have access to in a real game.
    fn update_state(&mut self, visible_state: VisibleState);

    /// Ask the player what they want to do.
    ///
    /// They can return None to continue to the breathing phase.
    fn play_action(&mut self) -> Option<Action>;

    /// This will only be asked if the player has a choice.
    fn breathe_or_travel(&mut self) -> BreatheOrTravel;

    /// This is only called if the defender can defend against the attack.
    fn defend(&mut self, action: &Action) -> bool;

    /// When a meteoroid hits the player they might have to discard.
    fn forced_discard(&mut self, card_count: usize) -> Vec<GameCard>;
}

pub struct RandomPlayerController {
    rng: ChaCha8Rng,
    visible_state: VisibleState,
}

impl RandomPlayerController {
    pub fn new() -> Self {
        Self {
            rng: ChaCha8Rng::from_entropy(),
            visible_state: VisibleState::invalid(),
        }
    }
}

impl PlayerController for RandomPlayerController {
    fn update_state(&mut self, visible_state: VisibleState) {
        self.visible_state = visible_state;
    }

    fn play_action(&mut self) -> Option<Action> {
        random_action(&self.visible_state)
    }

    fn breathe_or_travel(&mut self) -> BreatheOrTravel {
        if self.rng.gen() {
            BreatheOrTravel::Breathe
        } else {
            BreatheOrTravel::Travel
        }
    }

    fn defend(&mut self, action: &Action) -> bool {
        true
    }

    fn forced_discard(&mut self, card_count: usize) -> Vec<GameCard> {
        let mut cards = self.visible_state.my_hand.clone();
        cards.shuffle(&mut self.rng);
        cards.truncate(card_count);
        cards
    }
}

fn random_action(visible_state: &VisibleState) -> Option<Action> {
    let attackable_players = visible_state
        .players
        .iter()
        .enumerate()
        .map(|(player_reference, player)| (PlayerReference(player_reference), player))
        // Don't try to attack myself.
        .filter(|(player_reference, _)| player_reference != &visible_state.whose_turn)
        // Only attack people with cards.
        .filter(|(_, player)| player.hand_size > 0)
        .collect::<Vec<_>>();

    let attack_player = match attackable_players.first() {
        Some((player_reference, _)) => player_reference,
        None => return None,
    }
    .to_owned();

    for card in &visible_state.my_hand {
        match card {
            GameCard::TractorBeam => {
                return Some(Action::TractorBeam {
                    other_player_reference: attack_player,
                });
            }
            // Can't use as an action.
            GameCard::O1 => {}
            // Can't use as an action.
            GameCard::O2 => {}
            // TODO!
            _ => {
                println!("RandomPlayerController doesn't know how to play {:?}", card);
            }
        }
    }

    None
}
