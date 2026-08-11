use multisweeper_core::{GameAction, GameDifficulty};

use crate::session::{PlayerAddr, PlayerId};

pub enum PlayerCommand {
    Join { handle: PlayerAddr },
    Leave,
    StartGame { difficulty: GameDifficulty },
    GameAction { action: GameAction },
    GameQuery,
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub command: PlayerCommand,
}
