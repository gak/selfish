use crate::{Action, GameCard, GameDeck, Player, SpaceDeck};

#[derive(Debug, PartialEq)]
pub struct PlayerReference(pub usize);

pub struct Game {
    game_deck: GameDeck,
    space_deck: SpaceDeck,
    players: Vec<Player>,
    pub whose_turn: PlayerReference,
    phase: Phase,
}

impl Game {
    pub fn new(player_count: usize) -> Game {
        let mut game_deck = GameDeck::shuffled();
        let mut space_deck = SpaceDeck::shuffled();

        let mut players = Vec::new();
        for _ in 0..player_count {
            let mut player = Player::new();

            player.give(game_deck.take(GameCard::O2));
            for _ in 0..4 {
                player.give(game_deck.take(GameCard::O1));
            }

            players.push(player);
        }

        Game {
            game_deck,
            space_deck,
            players,
            whose_turn: PlayerReference(0),
            phase: Phase::Pickup,
        }
    }

    pub fn print(&self) {
        for p in &self.players {
            println!("{:?}", p);
        }
    }

    pub fn current_player(&mut self) -> &mut Player {
        &mut self.players[self.whose_turn.0]
    }

    pub fn draw_card(&mut self) {
        assert_eq!(self.phase, Phase::Pickup);

        let card = self.game_deck.draw();
        self.current_player().give(card);

        self.phase = Phase::Actions;
    }

    pub fn action(&mut self, action: Action) -> miette::Result<()> {
        assert_eq!(self.phase, Phase::Actions);

        match action {
            Action::TractorBeam { other_player } => {
                // You can't tractor beam yourself!
                assert!(other_player != self.whose_turn);

                // Make sure the other player has a card to give.
                let other_player = &mut self.players[other_player.0];
                assert!(other_player.has_at_least_one_card());

                // Remove and make sure the player has the card. Will panic if not.
                self.current_player().remove_card(GameCard::TractorBeam);

                let other_player = &mut self.players[other_player.0];
                let card = other_player.random_card();
                self.current_player().give(card);
            }
        }

        self.phase = Phase::Pickup;
    }
}

#[derive(Debug, PartialEq)]
enum Phase {
    Pickup,
    Actions,
    Breath,
}
