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

#[derive(Clone)]
pub struct RequestContext {
    pub message_id: MessageId,
    pub reply_to: PlayerAddr,
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub request: RequestContext,
    pub command: PlayerCommand,
}
