use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

use crate::ws::{PlayerId, protocol::ServerMessage};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

pub enum LobbyCommand {
    AddPlayer { id: PlayerId, msg_sdr: mpsc::Sender<ServerMessage> },
    RemovePlayer(PlayerId),
    StartGame
}

struct Lobby {
    players: Vec<PlayerId>,
    host_id: PlayerId,
    status: LobbyStatus
}