use crate::actions::BreatheOrTravel;
use crate::errors::SelfishError;
use crate::player_controller::PlayerController;
use crate::visible_state::VisibleState;
use crate::{Action, GameCard, GameDeck, Player, SpaceCard, SpaceDeck};
use miette::{bail, WrapErr};
use owo_colors::{CssColors, DynColors, OwoColorize};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

        let space_deck = SpaceDeck::shuffled(&mut rng);
        let mut game_deck = GameDeck::shuffled(&mut rng);
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
            if !self.current_player().alive {
                continue;
            }

            // Always start at the pickup phase.
            let card = self.draw_card_phase();
            self.log(format!("Player picked up a {:?}.", card));

            // Keep asking the controller for an action until they don't want to do any more.
            loop {
                if self.current_player().in_solar_flare() {
                    break;
                }

                let visible_state = self.visible_state()?;
                let controller = self.current_controller()?;
                controller.update_state(visible_state);
                let action = match controller.play_action() {
                    None => {
                        break;
                    }
                    Some(action) => action,
                };

                if let Err(err) = self.action(action) {
                    self.log(format!("Controller tried to do an invalid action: {}", err));

                    // We are not patient enough for controllers that don't know how to play, so
                    // let us immediately move to the BreatheOrTravel phase.
                    break;
                }
            }

            self.breathe_or_travel()?;
        }
    }

    fn visible_state(&self) -> miette::Result<VisibleState> {
        VisibleState::try_from_game(self)
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
                Some(self.current_controller()?.breathe_or_travel())
            }
        };

        // Player is re-borrowed here because the controller needed it.
        let player = self.player_mut(&whose_turn_reference)?;
        match breathe_or_travel {
            Some(BreatheOrTravel::Breathe) => {
                player.remove_card(&GameCard::O1).wrap_err("Breathing.")?;
                self.game_deck.add_to_discard(GameCard::O1);
                self.log("Player breathed.".to_string());
            }
            Some(BreatheOrTravel::Travel) => {
                player.remove_card(&GameCard::O2).wrap_err("Travelling.")?;
                self.log("Player travelled.".to_string());
                self.add_space()?;
            }
            None => {
                self.player_died(&whose_turn_reference)?;
            }
        }

        self.phase = Phase::Pickup;
        self.next_player();

        Ok(())
    }

    pub fn player_died(&mut self, player_reference: &PlayerReference) -> miette::Result<()> {
        let player = self.player_mut(player_reference)?;
        player.alive = false;
        self.log(format!("Player {} died.", player_reference.0));
        self.next_player();
        Ok(())
    }

    pub fn add_space(&mut self) -> miette::Result<()> {
        let whose_turn_reference = self.whose_turn_reference;
        let space_card = self.space_deck.draw();

        let player = self.player_mut(&whose_turn_reference)?;
        player.space.push(space_card.clone());

        match &space_card {
            SpaceCard::BlankSpace => {
                self.log("Player got blank space.".to_string());
            }
            SpaceCard::UsefulJunk => {
                let card = self.draw_card();
                self.log(format!(
                    "Player got {:?} and picked up a {:?}.",
                    space_card, card,
                ));
            }
            SpaceCard::MysteriousNebula => {
                let card_1 = self.draw_card();
                let card_2 = self.draw_card();
                self.log(format!(
                    "Player got {:?} and picked up a {:?} and a {:?}.",
                    space_card, card_1, card_2
                ));
            }
            SpaceCard::Hyperspace => {
                self.log("Player got a hyperspace jump.".to_string());
                self.add_space()?;
            }
            SpaceCard::Meteoroid => {
                if self.current_player().hand.len() > 6 {
                    // TODO: Ask the controller to discard two cards.
                    let controller = self.current_controller()?;
                    let cards = controller.forced_discard(2);
                    if cards.len() != 2 {
                        return Err(SelfishError::InvalidDiscardCount {
                            expected: 2,
                            actual: cards.len(),
                        }
                        .into());
                    }
                    let player = self.current_player();
                    for card in &cards {
                        player.remove_card(card).wrap_err("Meteoroid.")?;
                    }

                    self.log(format!(
                        "Player got hit by a meteoroid and had to discard {:?}.",
                        cards
                    ));
                } else {
                    self.log("Player got hit by a meteoroid but had 6 or less cards.".to_string());
                }
            }
            SpaceCard::CosmicRadiation => {
                println!("TODO: Cosmic radiation");
            }
            SpaceCard::AsteroidField => {
                println!("TODO: Asteroid field");
            }
            SpaceCard::GravitationalAnomaly => {
                self.log(
                    "Player got a gravitational anomaly and moved back one space.".to_string(),
                );
                let player = self.current_player();
                player.space.pop();
            }
            SpaceCard::WormHole => {
                println!("TODO: Wormhole");
            }
            SpaceCard::SolarFlare => {
                // Nothing happens except that they can't use action cards.
            }
        }

        Ok(())
    }

    pub fn color(&self, player_reference: &PlayerReference) -> DynColors {
        let colors: [DynColors; 6] = [
            "#B83AF1", "#6EB122", "#DAAC06", "#00938A", "#E23838", "#A23450",
        ]
        .map(|color| color.parse().unwrap());

        colors[player_reference.0]
    }

    pub fn log(&self, note: String) {
        println!("\n{}", note.color(self.color(&self.whose_turn_reference)));
        self.print();
    }

    pub fn print(&self) {
        for (idx, player) in self.players.iter().enumerate() {
            let is_turn = self.whose_turn_reference == PlayerReference(idx);
            let prefix = if !player.alive {
                " x "
            } else if is_turn {
                "-->"
            } else {
                "   "
            };

            let color = if !player.alive {
                CssColors::OrangeRed
            } else if is_turn {
                CssColors::White
            } else {
                CssColors::Grey
            };
            println!(" {} {:?}", prefix, player.color(color));
        }
    }

    pub fn player_mut(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<&mut Player> {
        match self.players.get_mut(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(*player_reference,));
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
                bail!(SelfishError::PlayerDoesNotExist(*player_reference,));
            }
            Some(player) => Ok((player, &mut self.rng)),
        }
    }

    pub fn player(&self, player_reference: &PlayerReference) -> miette::Result<&Player> {
        match self.players.get(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(*player_reference));
            }
            Some(player) => Ok(player),
        }
    }

    pub fn current_player(&mut self) -> &mut Player {
        &mut self.players[self.whose_turn_reference.0]
    }

    pub fn current_controller(&mut self) -> miette::Result<&mut Box<dyn PlayerController>> {
        let player_reference = self.whose_turn_reference;
        self.controller(&player_reference)
    }

    pub fn controller(
        &mut self,
        player_reference: &PlayerReference,
    ) -> miette::Result<&mut Box<dyn PlayerController>> {
        match self.controllers.get_mut(player_reference.0) {
            None => {
                bail!(SelfishError::PlayerDoesNotExist(*player_reference,));
            }
            Some(controller) => Ok(controller),
        }
    }

    pub fn draw_card_phase(&mut self) -> GameCard {
        assert_eq!(self.phase, Phase::Pickup);
        let card = self.draw_card();
        self.phase = Phase::Actions;
        card
    }

    fn draw_card(&mut self) -> GameCard {
        let card = self.game_deck.draw(&mut self.rng);
        self.current_player().give(card);
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
                self.log(format!(
                    "{:?} defended against {:?} with a shield.",
                    other_player_reference, action
                ));
                self.player_mut(&other_player_reference)?
                    .remove_card(&GameCard::Shield)
                    .wrap_err("Controller requested to defend with shield.")?;
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
        self.current_player()
            .remove_card(card)
            .wrap_err_with(|| format!("Discarding {:?}", &card))?;
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
        game.draw_card_phase();
        game.action(Action::TractorBeam {
            other_player_reference: PlayerReference(1),
        })
        .unwrap();
        game.print();
        assert_eq!(game.player(&PlayerReference(0)).unwrap().hand.len(), 6);
        assert_eq!(game.player(&PlayerReference(1)).unwrap().hand.len(), 4);
    }
}
