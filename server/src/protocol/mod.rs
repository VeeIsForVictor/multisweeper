use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateLobby,
    JoinLobby { code: String },
    StartGame
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    LobbyState { players: Vec<String>, host_id: String },
    GameStarted,
    Error(String)
}