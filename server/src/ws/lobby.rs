use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;

use crate::ws::{PlayerId, protocol::{LobbyCommand, PlayerConnection}};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

pub struct Lobby {
    players: HashMap<PlayerId, PlayerConnection>,
    host_id: PlayerId,
    status: LobbyStatus
}

impl Lobby {
    pub fn new(host_id: PlayerId, host_connection: PlayerConnection) -> Self {
        let mut lobby = Lobby {
            players: HashMap::new(),
            host_id: host_id.clone(),
            status: LobbyStatus::Waiting
        };

        lobby.players.insert(host_id, host_connection);

        return lobby;
    }
}