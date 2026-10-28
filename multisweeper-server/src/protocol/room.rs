use multisweeper_core::{GameAction, GameDifficulty};

use crate::protocol::session::MessageId;
use crate::session::{PlayerAddr, PlayerId};

pub enum PlayerCommand {
    Join {
        resume_token: String,
        generation: u64,
    },
    Reattach {
        resume_token: String,
        generation: u64,
    },
    Detach {
        resume_token: String,
        generation: u64,
    },
    Leave,
    StartGame {
        difficulty: GameDifficulty,
    },
    GameAction {
        action: GameAction,
    },
    GameQuery,
}

#[derive(Clone)]
pub struct RequestContext {
    pub message_id: MessageId,
    pub reply_to: PlayerAddr,
}

pub struct RoomMessage {
    pub id: PlayerId,
    pub generation: u64,
    pub request: RequestContext,
    pub command: PlayerCommand,
}
