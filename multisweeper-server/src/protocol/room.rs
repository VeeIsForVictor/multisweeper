use multisweeper_core::{GameAction, GameDifficulty};

use crate::protocol::session::MessageId;
use crate::session::{PlayerAddr, PlayerId};

pub enum PlayerCommand {
    Join,
    Leave,
    StartGame { difficulty: GameDifficulty },
    GameAction { action: GameAction },
    GameQuery,
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub message_id: MessageId,
    pub reply_to: PlayerAddr,
    pub command: PlayerCommand,
}
