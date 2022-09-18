use crate::actions::Action;
use crate::game::PlayerReference;
use crate::player_controller::{PlayerController, RandomPlayerController};
use crate::space_cards::{SpaceCard, SpaceDeck};
use game::Game;
use game_cards::{GameCard, GameDeck};
use player::Player;
use rand::prelude::SliceRandom;

mod actions;
mod errors;
mod game;
mod game_cards;
mod player;
mod player_controller;
mod space_cards;

fn main() -> miette::Result<()> {
    let mut controllers: Vec<Box<dyn PlayerController>> = Vec::new();
    for _ in 0..4 {
        controllers.push(Box::new(RandomPlayerController::new()));
    }
    let seed = Some(0);
    let mut game = Game::new(seed, controllers);

    println!("\nNEW GAME!");
    game.print();

    println!("\n{:?}", game.whose_turn_reference);
    game.draw_card();
    game.print();

    game.action(Action::TractorBeam {
        other_player_reference: PlayerReference(1),
    })?;
    game.print();

    Ok(())
}
