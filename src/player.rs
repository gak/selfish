use crate::errors::SelfishError;
use crate::{GameCard, SpaceCard};
use miette::bail;
use rand::Rng;

#[derive(Debug)]
pub struct Player {
    pub alive: bool,
    pub hand: Vec<GameCard>,

    /// Cards are pushed to the end as they come in.
    pub space: Vec<SpaceCard>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            alive: true,
            hand: Vec::new(),
            space: Vec::new(),
        }
    }

    pub fn give(&mut self, card: GameCard) {
        self.hand.push(card);
    }

    pub fn has_card(&self, card: &GameCard) -> bool {
        self.hand.contains(card)
    }

    /// Remove a card from the player's hand.
    pub fn remove_card(&mut self, card: &GameCard) -> miette::Result<()> {
        let index = self
            .hand
            .iter()
            .position(|c| c == card)
            .ok_or(SelfishError::PlayerDoesNotHaveThisCard(*card))?;

        self.hand.remove(index);

        Ok(())
    }

    /// Will return an error if the player does not have any cards.
    pub fn remove_random_card(&mut self, rng: &mut impl Rng) -> miette::Result<GameCard> {
        if self.hand.len() == 0 {
            bail!(SelfishError::PlayerHasNoCardsLeft);
        }

        let index = rng.gen_range(0..self.hand.len());
        Ok(self.hand.remove(index))
    }

    pub fn last_space_card(&self) -> Option<SpaceCard> {
        self.space.last().cloned()
    }

    pub fn in_solar_flare(&self) -> bool {
        self.last_space_card() == Some(SpaceCard::SolarFlare)
    }
}
