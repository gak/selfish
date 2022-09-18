use rand::prelude::SliceRandom;
use rand::Rng;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum GameCard {
    O1,
    O2,
    OxygenSiphon,
    Shield,
    HackSuit,
    TractorBeam,
    RocketBooster,
    LaserBlast,
    HoleInSuit,
    Tether,
}

#[derive(Debug)]
pub struct GameDeck {
    available: Vec<GameCard>,
    discard: Vec<GameCard>,
}

impl GameDeck {
    pub fn new() -> Self {
        let mut available = Vec::new();
        for _ in 0..38 {
            available.push(GameCard::O1);
        }
        for _ in 0..10 {
            available.push(GameCard::O2);
        }
        for _ in 0..10 {
            available.push(GameCard::O2);
        }
        for _ in 0..3 {
            available.push(GameCard::OxygenSiphon);
        }
        for _ in 0..4 {
            available.push(GameCard::Shield);
        }
        for _ in 0..3 {
            available.push(GameCard::HackSuit);
        }
        for _ in 0..4 {
            available.push(GameCard::TractorBeam);
        }
        for _ in 0..4 {
            available.push(GameCard::RocketBooster);
        }
        for _ in 0..4 {
            available.push(GameCard::LaserBlast);
        }
        for _ in 0..4 {
            available.push(GameCard::HoleInSuit);
        }
        for _ in 0..4 {
            available.push(GameCard::Tether);
        }

        Self {
            available,
            discard: Vec::new(),
        }
    }

    pub fn shuffled(rng: &mut impl Rng) -> Self {
        let mut deck = Self::new();
        deck.available.shuffle(rng);
        deck
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.available.shuffle(rng);
    }

    /// Used for initial deal only. Will panic if the card isn't in the deck!
    pub fn take(&mut self, card: GameCard) -> GameCard {
        let idx = self.available.iter().position(|c| *c == card).unwrap();
        self.available.remove(idx)
    }

    /// If there are no cards left, move the discard pile into the available pile and shuffle.
    pub fn draw(&mut self, rng: &mut impl Rng) -> GameCard {
        if self.available.is_empty() {
            self.available.append(&mut self.discard);
            self.available.shuffle(rng);
            self.discard.clear();
        }
        self.available.pop().unwrap()
    }

    pub fn add_to_discard(&mut self, card: GameCard) {
        self.discard.push(card);
    }

    // Used for cheating in tests!
    #[cfg(test)]
    pub fn add_to_available(&mut self, card: GameCard) {
        self.available.push(card);
    }
}
