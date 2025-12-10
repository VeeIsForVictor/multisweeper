use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::ws::PlayerId;
use super::lobby::LobbyStatus;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateLobby,
    JoinLobby { code: String },
    StartGame
}

pub enum LobbyCommand {
    AddPlayer { id: PlayerId, msg_sdr: mpsc::Sender<ServerMessage> },
    RemovePlayer(PlayerId),
    StartGame
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    LobbyState { players: Vec<PlayerId>, host_id: PlayerId, status: LobbyStatus },
    GameStarted,
    Error(String)
}