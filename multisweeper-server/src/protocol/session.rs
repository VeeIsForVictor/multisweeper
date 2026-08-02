use serde::Serialize;

use crate::{room::RoomCode, session::PlayerId};

#[derive(Serialize)]
pub enum SessionMessage {
    RoomState {
        code: RoomCode,
        owner: PlayerId,
        players: Vec<PlayerId>,
    },
    GameStarted,
}
