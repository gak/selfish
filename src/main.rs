use crate::actions::Action;
use crate::game::PlayerReference;
use crate::player_controller::{PlayerController, RandomPlayerController};
use crate::space_cards::{SpaceCard, SpaceDeck};
use game::Game;
use game_cards::{GameCard, GameDeck};
use player::Player;
use rand::{thread_rng, Rng};

mod actions;
mod errors;
mod game;
mod game_cards;
mod player;
mod player_controller;
mod space_cards;
mod visible_state;

fn main() -> miette::Result<()> {
    let mut controllers: Vec<Box<dyn PlayerController>> = Vec::new();
    for _ in 0..4 {
        controllers.push(Box::new(RandomPlayerController::new()));
    }
    let seed = Some(thread_rng().gen());
    let mut game = Game::new(seed, controllers);
    game.simulate()?;

    Ok(())
}
