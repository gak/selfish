use crate::game::PlayerReference;

pub enum Action {
    TractorBeam { other_player: PlayerReference }
}