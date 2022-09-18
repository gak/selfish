use crate::actions::BreatheOrTravel;
use crate::errors::SelfishError;
use crate::player_controller::PlayerController;
use crate::{Action, GameCard, GameDeck, Player, SpaceCard, SpaceDeck};
use miette::bail;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PlayerReference(pub usize);

pub struct Game {
    rng: ChaCha8Rng,
    game_deck: GameDeck,
    space_deck: SpaceDeck,
    players: Vec<Player>,
    controllers: Vec<Box<dyn PlayerController>>,
    pub whose_turn_reference: PlayerReference,
    phase: Phase,
}

impl Game {
    pub fn new(seed: Option<u64>, controllers: Vec<Box<dyn PlayerController>>) -> Game {
        let mut rng = match seed {
            None => ChaCha8Rng::from_entropy(),
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
        };

        let mut game_deck = GameDeck::shuffled(&mut rng);
        let mut space_deck = SpaceDeck::shuffled(&mut rng);

        let mut players = Vec::new();
        for _ in 0..controllers.len() {
            let mut player = Player::new();

            player.give(game_deck.take(GameCard::O2));
            for _ in 0..4 {
                player.give(game_deck.take(GameCard::O1));
            }

            players.push(player);
        }

        Game {
            rng,
            game_deck,
            space_deck,
            players,
            controllers,
            whose_turn_reference: PlayerReference(0),
            phase: Phase::Pickup,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn simulate(&mut self) -> miette::Result<()> {
        loop {
            // Always start at the pickup phase.
            let card = self.draw_card();
            println!("Player picked up a {:?}", card);
            self.print();

            // Keep asking the controller for an action until they don't want to do any more.
            loop {
                let controller = self.controller(&self.whose_turn_reference.clone())?;
                let action = match controller.play_action() {
                    None => {
                        break;
                    }
                    Some(action) => action,
                };

                if let Err(err) = self.action(action) {
                    println!("Controller tried to do an invalid action: {}", err);

                    // We are not patient enough for controllers that don't know how to play, so
                    // let us immediately move to the BreatheOrTravel phase.
                    break;
                }
            }

            self.breathe_or_travel()?;
        }
    }

    fn next_player(&mut self) {
        let next_player = (self.whose_turn_reference.0 + 1) % self.players.len();
        self.whose_turn_reference = PlayerReference(next_player);
    }

    /// If the player has no oxygen cards, they die.
    /// If the player only has either an O1 or an O2, they are automatically played without
    /// asking the controller.
    /// Otherwise, ask the controller to play a O1 or a O2.
    pub fn breathe_or_travel(&mut self) -> miette::Result<()> {
        assert_eq!(self.phase, Phase::Actions);
        self.phase = Phase::BreatheOrTravel;

        let whose_turn_reference = self.whose_turn_reference;

        let player = self.player_mut(&whose_turn_reference)?;
        let has_o1 = player.has_card(&GameCard::O1);
        let has_o2 = player.has_card(&GameCard::O2);

        let breathe_or_travel: Option<BreatheOrTravel> = match (has_o1, has_o2) {
            (false, false) => None,
            (true, false) => Some(BreatheOrTravel::Breathe),
            (false, true) => Some(BreatheOrTravel::Travel),
            (true, true) => {
                // The player has both an O1 and an O2, so ask the controller to play one.
                Some(self.controller(&whose_turn_reference)?.breathe_or_travel())
            }
        };

        println!("Player is going to {:?}", breathe_or_travel);
        self.print();

        // Player is re-borrowed here because the controller needed it.
        let player = self.player_mut(&whose_turn_reference)?;
        match breathe_or_travel {
            Some(BreatheOrTravel::Breathe) => {
                player.remove_card(&GameCard::O1)?;
                self.game_deck.add_to_discard(GameCard::O1);
            }
            Some(BreatheOrTravel::Travel) => {
                // The player only has an O2, so play it.
                player.remove_card(&GameCard::O2)?;
                // player.space.push(self.space_deck.take());
                println!("TODO: Deal with space");
            }
            None => {
                // The player is dead.
                todo!("TODO: Deal with dead player");
            }
        }

        self.phase = Phase::Pickup;
        self.next_player();

        Ok(())
    }

    pub fn print(&self) {
        for (idx, p) in self.players.iter().enumerate() {
            let turn = if self.whose_turn_reference == PlayerReference(idx) {
                " * "
            } else {
                "   "
            };
            println!("{} {:?}", turn, p);
        }
    }

    pub fn player_mut(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<&mut Player> {
        match self.players.get_mut(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(player_reference.clone(),));
            }
            Some(player) => Ok(player),
        }
    }

    /// This subverts the borrow checker in a safe via a splitting borrow.
    pub fn player_mut_rng(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<(&mut Player, &mut ChaCha8Rng)> {
        match self.players.get_mut(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(player_reference.clone(),));
            }
            Some(player) => Ok((player, &mut self.rng)),
        }
    }

    pub fn player(&self, player_reference: &PlayerReference) -> miette::Result<&Player> {
        match self.players.get(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(player_reference.clone(),));
            }
            Some(player) => Ok(player),
        }
    }

    pub fn current_player(&mut self) -> &mut Player {
        &mut self.players[self.whose_turn_reference.0]
    }

    pub fn controller(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<&mut Box<dyn PlayerController>> {
        match self.controllers.get_mut(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(player_reference.clone(),));
            }
            Some(controller) => Ok(controller),
        }
    }

    pub fn draw_card(&mut self) -> GameCard {
        assert_eq!(self.phase, Phase::Pickup);

        let card = self.game_deck.draw(&mut self.rng);
        self.current_player().give(card);

        self.phase = Phase::Actions;

        card
    }

    pub fn action(&mut self, action: Action) -> miette::Result<()> {
        assert_eq!(self.phase, Phase::Actions);

        let mut proceed = true;

        if let Some(other_player_reference) = action.attacking() {
            if other_player_reference == self.whose_turn_reference {
                bail!(SelfishError::CantAttackYourself);
            }

            let other_player = self.player_mut(&other_player_reference)?;
            if let Some(steal_count) = action.stealing() {
                if steal_count > other_player.hand.len() {
                    bail!(SelfishError::PlayerDoesNotHaveEnoughCards(
                        other_player_reference,
                        steal_count
                    ));
                }
            }

            // Offer the other player a chance to shield.
            if self.can_player_defend(&other_player_reference)?
                && self.controller(&other_player_reference)?.defend(&action)
            {
                self.player_mut(&other_player_reference)?
                    .remove_card(&GameCard::Shield)?;
                proceed = false;
            }
        }

        if proceed {
            match action {
                Action::TractorBeam {
                    other_player_reference,
                } => {
                    let random_card = self.remove_random_card(&other_player_reference)?;
                    self.current_player().give(random_card);
                }
            }
        }

        // Remove action card from the player's hand.

        self.discard(&action.card())?;

        Ok(())
    }

    pub fn discard(&mut self, card: &GameCard) -> miette::Result<()> {
        self.current_player().remove_card(card)?;
        self.game_deck.add_to_discard(GameCard::O1);
        Ok(())
    }

    pub fn can_player_defend(&self, player_reference: &PlayerReference) -> miette::Result<bool> {
        let player = self.player(player_reference)?;
        let has_shield_card = player.hand.contains(&GameCard::Shield);
        let in_solar_flare = player.in_solar_flare();
        Ok(has_shield_card && !in_solar_flare)
    }

    fn remove_random_card(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<GameCard> {
        let (other_player, rng) = self.player_mut_rng(&player_reference)?;
        Ok(other_player.remove_random_card(rng)?)
    }
}

#[derive(Debug, PartialEq)]
enum Phase {
    Pickup,
    Actions,
    BreatheOrTravel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RandomPlayerController;
    use rand::{thread_rng, Rng};

    fn new_game(players: usize) -> Game {
        let seed = Some(thread_rng().gen());
        println!("Seed: {:?}", seed);

        let mut controllers: Vec<Box<dyn PlayerController>> = Vec::new();
        for _ in 0..players {
            controllers.push(Box::new(RandomPlayerController::new()));
        }
        Game::new(seed, controllers)
    }

    #[test]
    fn test_tractor_beam() {
        let mut game = new_game(2);
        // Cheat and put a tractor beam on the top of the deck.
        game.game_deck.add_to_available(GameCard::TractorBeam);
        game.draw_card();
        game.action(Action::TractorBeam {
            other_player_reference: PlayerReference(1),
        })
        .unwrap();
        game.print();
        assert_eq!(game.player(&PlayerReference(0)).unwrap().hand.len(), 6);
        assert_eq!(game.player(&PlayerReference(1)).unwrap().hand.len(), 4);
    }
}
