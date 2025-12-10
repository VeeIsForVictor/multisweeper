use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::ws::{PlayerId, protocol::PlayerConnection};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

struct Lobby {
    players: HashMap<PlayerId, PlayerConnection>,
    host_id: PlayerId,
    status: LobbyStatus
}