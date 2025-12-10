use serde::{Deserialize, Serialize};
use super::lobby::LobbyStatus;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateLobby,
    JoinLobby { code: String },
    StartGame
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    LobbyState { players: Vec<String>, host_id: String, status: LobbyStatus },
    GameStarted,
    Error(String)
}