use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

use crate::ws::{PlayerId, protocol::ServerMessage};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

struct Lobby {
    players: Vec<PlayerId>,
    host_id: PlayerId,
    status: LobbyStatus
}