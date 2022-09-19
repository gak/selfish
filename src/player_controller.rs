use crate::actions::BreatheOrTravel;
use crate::visible_state::{VisiblePlayer, VisibleState};
use crate::{Action, GameCard, PlayerReference};
use rand::prelude::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

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

    /// Wormholes make you swap with another player.
    fn choose_player_to_swap_with(&mut self) -> PlayerReference;

    /// Hack suit to choose a card to steal.
    fn choose_card_to_take(&mut self, options: HashSet<GameCard>) -> GameCard;
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

    fn defend(&mut self, _action: &Action) -> bool {
        true
    }

    fn forced_discard(&mut self, card_count: usize) -> Vec<GameCard> {
        let mut cards = self.visible_state.my_hand.clone();
        cards.shuffle(&mut self.rng);
        cards.truncate(card_count);
        cards
    }

    fn choose_player_to_swap_with(&mut self) -> PlayerReference {
        let targets = potential_targets(&self.visible_state, false, 0, false);
        let target = targets.choose(&mut self.rng).unwrap();
        *target
    }

    fn choose_card_to_take(&mut self, options: HashSet<GameCard>) -> GameCard {
        *options.iter().next().unwrap()
    }
}

fn potential_targets(
    visible_state: &VisibleState,
    needs_to_be_alive: bool,
    needs_cards: usize,
    needs_to_have_space: bool,
) -> Vec<PlayerReference> {
    let mut found = Vec::new();
    for (idx, player) in visible_state.players.iter().enumerate() {
        let player_reference = PlayerReference(idx);

        if player_reference == visible_state.whose_turn {
            continue;
        }
        if needs_to_be_alive && !player.alive {
            continue;
        }
        if needs_cards > 0 && player.hand_size < needs_cards {
            continue;
        }
        if needs_to_have_space && player.space.is_empty() {
            continue;
        }

        found.push(player_reference);
    }

    found
}

fn random_action(visible_state: &VisibleState) -> Option<Action> {
    let targets = potential_targets(visible_state, true, 1, false);
    let target = match targets.first() {
        Some(player_reference) => player_reference,
        None => return None,
    }
    .to_owned();

    for card in &visible_state.my_hand {
        match card {
            GameCard::OxygenSiphon => {
                return Some(Action::OxygenSiphon { target });
            }
            GameCard::HackSuit => {
                return Some(Action::HackSuit { target });
            }
            GameCard::TractorBeam => {
                return Some(Action::TractorBeam { target });
            }
            GameCard::RocketBooster => {
                return Some(Action::RocketBooster);
            }
            GameCard::LaserBlast => {
                let target = *potential_targets(visible_state, true, 1, true).first()?;
                return Some(Action::LaserBlast { target });
            }
            GameCard::HoleInSuit => {
                return Some(Action::HoleInSuit { target });
            }
            GameCard::Tether => {
                let target = *potential_targets(visible_state, true, 1, true).first()?;
                return Some(Action::Tether { target });
            }
            // Can't use as an action.
            GameCard::O1 => {}
            // Can't use as an action.
            GameCard::O2 => {}
            // Can't use as an action.
            GameCard::Shield => {}
        }
    }

    None
}
