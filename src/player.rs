use crate::{GameCard, SpaceCard};

#[derive(Debug)]
pub struct Player {
    hand: Vec<GameCard>,

    /// Cards are pushed to the end as they come in.
    space: Vec<SpaceCard>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            hand: Vec::new(),
            space: Vec::new(),
        }
    }

    pub fn give(&mut self, card: GameCard) {
        self.hand.push(card);
    }

    /// This will panic if the player doesn't have the card.
    pub fn remove_card(&mut self, card: GameCard) {
        let index = self.hand.iter().position(|c| *c == card).unwrap();
        self.hand.remove(index);
    }
}
