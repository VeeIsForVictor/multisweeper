use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;

use crate::ws::{PlayerId, protocol::{LobbyCommand, PlayerConnection, ServerMessage}};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    pub fn register_player(&mut self, player_id: PlayerId, player_connection: PlayerConnection) -> PlayerId {
        self.players.insert(player_id.clone(), player_connection);
        return player_id;
    }

    pub async fn broadcast_state(&mut self) {
        let players_list: Vec<PlayerId> = self.players.keys().cloned().collect();
        for player in self.players.values() {
            player.message_sdr.send(ServerMessage::LobbyState {
                players: players_list.clone(), 
                host_id: self.host_id.clone(), 
                status: self.status.clone()
            }).await;
        }
    }
}