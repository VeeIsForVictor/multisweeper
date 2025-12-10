use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::ws::protocol::ServerMessage;

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

pub enum LobbyCommand {
    AddPlayer { id: String, msg_sdr: mpsc::Sender<ServerMessage> },
    RemovePlayer(String),
    StartGame
}

struct Lobby {
    players: Vec<String>,
    host_id: String,
    status: LobbyStatus
}