use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Sender};
use crate::ws::PlayerId;
use super::lobby::LobbyStatus;

pub struct PlayerConnection {
    action_sdr: Sender<ClientMessage>,
    message_sdr: Sender<ServerMessage>
}

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