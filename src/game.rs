use crate::actions::BreatheOrTravel;
use crate::errors::SelfishError;
use crate::player_controller::PlayerController;
use crate::visible_state::VisibleState;
use crate::{Action, GameCard, GameDeck, Player, SpaceCard, SpaceDeck};
use miette::{bail, WrapErr};
use owo_colors::{CssColors, DynColors, OwoColorize};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::mem::swap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PlayerReference(pub usize);

pub struct Game {
    rng: ChaCha8Rng,
    game_over: bool,
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
            game_over: false,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn simulate(&mut self) -> miette::Result<()> {
        while !self.game_over {
            if !self.current_player().alive {
                self.next_player();
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

        Ok(())
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
                self.game_deck.add_to_discard(GameCard::O1);
                self.log("Player travelled.".to_string());
                self.add_space()?;
            }
            None => {
                self.player_died(
                    &whose_turn_reference,
                    "they didn't have any oxygen cards left.",
                )?;
            }
        }

        self.phase = Phase::Pickup;
        self.next_player();

        Ok(())
    }

    pub fn player_died(
        &mut self,
        player_reference: &PlayerReference,
        reason: &str,
    ) -> miette::Result<()> {
        let player = self.player_mut(player_reference)?;
        player.alive = false;
        self.log(format!(
            "Player {} died because {}.",
            player_reference.0, reason
        ));

        self.check_game_over();

        self.next_player();
        Ok(())
    }

    pub fn check_game_over(&mut self) {
        let mut alive_count = 0;
        for player in &self.players {
            if player.alive {
                alive_count += 1;
            }
        }

        println!("Alive count: {}", alive_count);

        if alive_count == 1 {
            self.log("Game over!".to_string());
            self.game_over = true;
        }
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
                    for card in &cards {
                        let player = self.current_player();
                        player.remove_card(card).wrap_err("Meteoroid.")?;
                        self.game_deck.add_to_discard(*card);
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
                // The player must discard an oxygen to survive.
                self.discard_or_die(GameCard::O1, "cosmic radiation")?;
            }
            SpaceCard::AsteroidField => {
                for _ in 0..2 {
                    self.discard_or_die(GameCard::O1, "asteroid field")?;
                }
            }
            SpaceCard::GravitationalAnomaly => {
                self.log(
                    "Player got a gravitational anomaly and moved back one space.".to_string(),
                );
                let player = self.current_player();
                player.space.pop();
            }
            SpaceCard::WormHole => {
                let controller = self.current_controller()?;
                let target_reference = controller.choose_player_to_swap_with();
                let whose_turn_reference = self.whose_turn_reference;
                self.swap_space(&whose_turn_reference, &target_reference)?;
                self.log(format!(
                    "Player got a wormhole and swapped spaces with player {}.",
                    target_reference.0
                ));
            }
            SpaceCard::SolarFlare => {
                // Nothing happens except that they can't use action cards.
            }
        }

        Ok(())
    }

    pub fn swap_space(&mut self, p1: &PlayerReference, p2: &PlayerReference) -> miette::Result<()> {
        let p1_space = self.player(p1)?.space.clone();
        let p2_space = self.player(p2)?.space.clone();

        self.player_mut(p1)?.space = p2_space;
        self.player_mut(p2)?.space = p1_space;

        Ok(())
    }

    pub fn discard_or_die(&mut self, card: GameCard, reason: &str) -> miette::Result<()> {
        let whose_turn_reference = self.whose_turn_reference;
        let player = self.current_player();

        // TODO: Automatically try to swap an O2 for two O1's.
        if player.has_card(&card) {
            player.remove_card(&card)?;
            self.game_deck.add_to_discard(card);
            self.log("Player survived cosmic radiation.".to_string());
        } else {
            self.player_died(&whose_turn_reference, reason)?;
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

        let rules = action.rules();

        if let Some(other_player_reference) = action.attacking() {
            if other_player_reference == self.whose_turn_reference {
                bail!(SelfishError::CantAttackYourself);
            }

            let other_player = self.player_mut(&other_player_reference)?;
            if let Some(steal) = action.rules().steal {
                if steal.count > other_player.hand.len() {
                    bail!(SelfishError::PlayerDoesNotHaveEnoughCards(
                        other_player_reference,
                        steal.count
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
                self.game_deck.add_to_discard(GameCard::Shield);
                proceed = false;
            }
        }

        if proceed {
            match action {
                Action::OxygenSiphon { target } => {
                    self.log(format!("Player will siphon oxygen from {:?}.", target));
                    let target_player = self.player_mut(&target)?;
                    match target_player.count_cards(&GameCard::O1) {
                        0 => {
                            self.player_died(
                                &target,
                                "was attacked by an oxygen siphon and didn't have enough oxygen",
                            )?;
                        }
                        1 => {
                            self.current_player().give(GameCard::O1);
                            self.player_died(
                                &target,
                                "was attacked by an oxygen siphon and only had 1 oxygen",
                            )?;
                        }
                        _ => {
                            target_player.remove_card(&GameCard::O1)?;
                            target_player.remove_card(&GameCard::O1)?;
                            self.current_player().give(GameCard::O1);
                        }
                    }
                }
                Action::HackSuit { target } => {
                    let target_player = self.player(&target)?;
                    let possible_cards = target_player.unique_cards();
                    let controller = self.current_controller()?;
                    let card = controller.choose_card_to_take(possible_cards);
                    let target_player = self.player_mut(&target)?;
                    target_player.remove_card(&card)?;
                    self.current_player().give(card);
                    self.log(format!(
                        "Player hacked {:?}'s suit and took a {:?}.",
                        target, card
                    ));
                }
                Action::TractorBeam { target } => {
                    let random_card = self.remove_random_card(&target)?;
                    self.current_player().give(random_card);
                }
                Action::RocketBooster => {
                    self.add_space()?;
                    self.log("Player used a rocket booster.".to_string());
                }
                Action::LaserBlast { target } => {
                    let target_player = self.player_mut(&target)?;
                    target_player.space.pop();
                }
                Action::HoleInSuit { .. } => {}
                Action::Tether { .. } => {}
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
        let (other_player, rng) = self.player_mut_rng(player_reference)?;
        other_player.remove_random_card(rng)
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
            target: PlayerReference(1),
        })
        .unwrap();
        game.print();
        assert_eq!(game.player(&PlayerReference(0)).unwrap().hand.len(), 6);
        assert_eq!(game.player(&PlayerReference(1)).unwrap().hand.len(), 4);
    }
}
