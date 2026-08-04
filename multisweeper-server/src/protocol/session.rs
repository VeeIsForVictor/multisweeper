use multisweeper_core::GameSnapshot;
use serde::Serialize;

use crate::{room::{RoomCode, RoomState}, session::PlayerId};

#[derive(Serialize, Clone)]
pub enum SessionMessage {
    RoomState {
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerId>,
        game: GameSnapshot
    },
    Kicked {
        reason: String
    },
    GameStarted,
}

impl From<RoomState> for SessionMessage {
    fn from(value: RoomState) -> Self {
        return SessionMessage::RoomState {
            code: value.code,
            owner: value.owner,
            players: value.players,
            game: value.game
        };
    }
}