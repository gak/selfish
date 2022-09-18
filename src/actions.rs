use crate::game::PlayerReference;
use crate::GameCard;

#[derive(Debug)]
pub enum BreatheOrTravel {
    Breathe,
    Travel,
}

#[derive(Debug)]
pub enum Action {
    TractorBeam {
        other_player_reference: PlayerReference,
    },
}

impl Action {
    pub fn card(&self) -> GameCard {
        match self {
            Action::TractorBeam { .. } => GameCard::TractorBeam,
        }
    }

    pub fn attacking(&self) -> Option<PlayerReference> {
        match self {
            Action::TractorBeam {
                other_player_reference,
            } => Some(*other_player_reference),
        }
    }

    pub fn stealing(&self) -> Option<usize> {
        match self {
            Action::TractorBeam { .. } => Some(1),
        }
    }
}
