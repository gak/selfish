use crate::actions::BreatheOrTravel;
use crate::Action;

pub trait PlayerController {
    /// Give the player only the information that they would have access to in a real game.
    ///
    /// * Whose turn it is
    /// * The number of cards in each player's hand
    /// * The space grid.
    // TODO: fn update_state(&mut self, state: &VisibleState);

    /// Ask the player what they want to do. They can return None to continue to the breathing
    /// stage.
    fn play_action(&mut self) -> Option<Action>;

    /// This will only be asked if the player has a choice, otherwise the game will do it for
    /// them.
    fn breathe_or_travel(&mut self) -> BreatheOrTravel;

    /// This is only called if the defender can defend against the attack.
    ///
    /// * They have a shield.
    /// * They are not in a nebula.
    fn defend(&mut self, action: &Action) -> bool;
}

pub struct RandomPlayerController;

impl RandomPlayerController {
    pub fn new() -> RandomPlayerController {
        RandomPlayerController
    }
}

impl PlayerController for RandomPlayerController {
    fn play_action(&mut self) -> Option<Action> {
        todo!()
    }

    fn breathe_or_travel(&mut self) -> BreatheOrTravel {
        todo!()
    }

    fn defend(&mut self, action: &Action) -> bool {
        todo!()
    }
}
