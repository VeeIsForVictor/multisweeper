use serde::Serialize;

use crate::{room::{RoomCode, RoomState}, session::PlayerId};

#[derive(Serialize, Clone)]
pub enum SessionMessage {
    RoomState {
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerId>,
    },
    Kicked,
    GameStarted,
}

impl From<RoomState> for SessionMessage {
    fn from(value: RoomState) -> Self {
        return SessionMessage::RoomState {
            code: value.code,
            owner: value.owner,
            players: value.players
        };
    }
}