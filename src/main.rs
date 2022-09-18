use rand::prelude::SliceRandom;
use rand::thread_rng;
use game::Game;
use game_cards::{GameCard, GameDeck};
use player::Player;
use crate::actions::Action;
use crate::game::PlayerReference;
use crate::space_cards::{SpaceCard, SpaceDeck};

mod game_cards;
mod space_cards;
mod actions;
mod player;
mod game;

fn main() -> miette::Result<()> {
    let mut game = Game::new(4);

    println!("\nNEW GAME!");
    game.print();

    println!("\n{:?}", game.whose_turn);
    game.draw_card();
    game.print();

    game.action(Action::TractorBeam { other_player: PlayerReference(1) })?;

    Ok(())
}

trait PlayerController {
    /// Give the player only the information that they would have access to in a real game.
    ///
    /// * Whose turn it is
    /// * The number of cards in each player's hand
    /// * The space grid.
    // TODO: fn update_state(&mut self, state: &VisibleState);

    fn play_action(&mut self) -> Option<Action>;

    /// This is only called if the defender can defend against the attack.
    ///
    /// * They have a shield.
    /// * They are not in a nebula.
    fn defend(&mut self, attacker: PlayerReference, action: Action) -> bool;
}