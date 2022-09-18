use rand::prelude::SliceRandom;
use rand::{Rng, SeedableRng};

#[derive(Debug, PartialEq, Clone)]
pub enum SpaceCard {
    BlankSpace,
    UsefulJunk,
    MysteriousNebula,
    Hyperspace,
    Meteoroid,
    CosmicRadiation,
    AsteroidField,
    GravitationalAnomaly,
    WormHole,
    SolarFlare,
}

pub struct SpaceDeck(Vec<SpaceCard>);

impl SpaceDeck {
    pub fn new() -> Self {
        let mut cards = Vec::new();

        for _ in 0..9 {
            cards.push(SpaceCard::BlankSpace);
        }
        for _ in 0..5 {
            cards.push(SpaceCard::UsefulJunk);
        }
        for _ in 0..2 {
            cards.push(SpaceCard::MysteriousNebula);
        }
        for _ in 0..1 {
            cards.push(SpaceCard::Hyperspace);
        }
        for _ in 0..4 {
            cards.push(SpaceCard::Meteoroid);
        }
        for _ in 0..6 {
            cards.push(SpaceCard::CosmicRadiation);
        }
        for _ in 0..2 {
            cards.push(SpaceCard::AsteroidField);
        }
        for _ in 0..4 {
            cards.push(SpaceCard::GravitationalAnomaly);
        }
        for _ in 0..4 {
            cards.push(SpaceCard::WormHole);
        }
        for _ in 0..5 {
            cards.push(SpaceCard::SolarFlare);
        }

        Self(cards)
    }

    pub fn shuffled<R: Rng>(rng: &mut R) -> Self {
        let mut deck = Self::new();
        deck.0.shuffle(rng);
        deck
    }
}
