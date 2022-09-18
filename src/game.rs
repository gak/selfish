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

    pub fn print(&self) {
        for p in &self.players {
            println!("{:?}", p);
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

    pub fn draw_card(&mut self) {
        assert_eq!(self.phase, Phase::Pickup);

        let card = self.game_deck.draw(&mut self.rng);
        self.current_player().give(card);

        self.phase = Phase::Actions;
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
                    .remove_card(GameCard::Shield)?;
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
        self.current_player().remove_card(action.card())?;

        self.phase = Phase::Pickup;

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
        let other_player = self.player_mut(&player_reference)?;
        Ok(other_player.remove_random_card(&mut self.rng)?)
    }
}

#[derive(Debug, PartialEq)]
enum Phase {
    Pickup,
    Actions,
    Breath,
}
