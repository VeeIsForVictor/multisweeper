use asyncapi_rust::schemars::JsonSchema;
use multisweeper_core::GameSnapshot;
use serde::Serialize;

use crate::{
    room::{RoomCode, RoomState},
    session::PlayerId,
};

#[derive(Debug, Serialize, Clone, JsonSchema)]
pub enum SessionMessage {
    RoomState {
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerId>,
        game: Option<GameSnapshot>,
    },
    Kicked {
        reason: String,
    },
    Error {
        reason: String,
    },
    GameStarted,
}

impl From<RoomState> for SessionMessage {
    fn from(value: RoomState) -> Self {
        SessionMessage::RoomState {
            code: value.code,
            owner: value.owner,
            players: value.players,
            game: value.game,
        }
    }
}
