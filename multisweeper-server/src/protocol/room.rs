use crate::session::{PlayerAddr, PlayerId};

pub enum PlayerCommand {
    Join { handle: PlayerAddr },
    Leave
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub command: PlayerCommand
}