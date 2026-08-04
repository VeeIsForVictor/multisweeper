use multisweeper_core::GameDifficulty;

use crate::session::{PlayerAddr, PlayerId};

pub enum PlayerCommand {
    Join { handle: PlayerAddr },
    Leave,
    StartGame { difficulty: GameDifficulty }
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub command: PlayerCommand,
}
