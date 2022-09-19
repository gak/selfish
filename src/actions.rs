use crate::game::PlayerReference;
use crate::GameCard;

#[derive(Debug)]
pub enum BreatheOrTravel {
    Breathe,
    Travel,
}

#[derive(Debug)]
pub enum Action {
    OxygenSiphon { target: PlayerReference },
    HackSuit { target: PlayerReference },
    TractorBeam { target: PlayerReference },
    RocketBooster,
    LaserBlast { target: PlayerReference },
    HoleInSuit { target: PlayerReference },
    Tether { target: PlayerReference },
}

impl Action {
    pub fn card(&self) -> GameCard {
        match self {
            Action::OxygenSiphon { .. } => GameCard::OxygenSiphon,
            Action::HackSuit { .. } => GameCard::HackSuit,
            Action::TractorBeam { .. } => GameCard::TractorBeam,
            Action::RocketBooster => GameCard::RocketBooster,
            Action::LaserBlast { .. } => GameCard::LaserBlast,
            Action::HoleInSuit { .. } => GameCard::HoleInSuit,
            Action::Tether { .. } => GameCard::Tether,
        }
    }

    pub fn attacking(&self) -> Option<PlayerReference> {
        match self {
            Action::OxygenSiphon { target } => Some(*target),
            Action::HackSuit { target } => Some(*target),
            Action::TractorBeam { target } => Some(*target),
            Action::RocketBooster => None,
            Action::LaserBlast { target } => Some(*target),
            Action::HoleInSuit { target } => Some(*target),
            Action::Tether { target } => Some(*target),
        }
    }

    pub fn rules(&self) -> ActionRules {
        match self {
            Action::OxygenSiphon { .. } => ActionRules {
                steal: Some(Steal {
                    count: 2,
                    visibility: StealAccess::Specific(GameCard::O1),
                }),
            },
            Action::HackSuit { .. } => ActionRules {
                steal: Some(Steal {
                    count: 1,
                    visibility: StealAccess::SeeCardsAndChoose,
                }),
            },
            Action::TractorBeam { .. } => ActionRules {
                steal: Some(Steal {
                    count: 1,
                    visibility: StealAccess::Random,
                }),
            },
            Action::RocketBooster => ActionRules::default(),
            Action::LaserBlast { .. } => ActionRules::default(),
            Action::HoleInSuit { .. } => ActionRules::default(),
            Action::Tether { .. } => ActionRules::default(),
        }
    }
}

#[derive(Default)]
pub struct ActionRules {
    pub steal: Option<Steal>,
}

pub struct Steal {
    pub count: usize,
    pub visibility: StealAccess,
}

pub enum StealAccess {
    Specific(GameCard),
    Random,
    SeeCardsAndChoose,
}
