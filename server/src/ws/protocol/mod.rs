use serde::{Deserialize, Serialize};
use crate::ws::PlayerId;
use super::lobby::LobbyStatus;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateLobby,
    JoinLobby { code: String },
    StartGame
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    LobbyState { players: Vec<PlayerId>, host_id: PlayerId, status: LobbyStatus },
    GameStarted,
    Error(String)
}