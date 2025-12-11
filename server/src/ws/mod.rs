use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::ws::{lobby::LobbyCode, protocol::{ClientMessage, LobbyCommand, PlayerConnection, ServerMessage}};

mod lobby;
pub mod protocol;

pub type PlayerId = String;

pub struct SharedState {
    lobbies: HashMap<LobbyCode, mpsc::Sender<LobbyCommand>>,
    idle_players: HashMap<PlayerId, PlayerConnection>,
    latest_player_id_number: u32
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            lobbies: HashMap::new(),
            idle_players: HashMap::new(),
            latest_player_id_number: 0
        }
    }

    pub fn register_player(&mut self, mut action_sdr: Sender<ClientMessage>, mut message_sdr: Sender<ServerMessage>) -> PlayerId {
        let connection = PlayerConnection { action_sdr: action_sdr, message_sdr: message_sdr };
        let player_id: PlayerId = format!("player {}", self.latest_player_id_number);
        self.latest_player_id_number += 1;
        self.idle_players.insert(player_id.clone(), connection);
        return player_id;
    }

    pub fn de_idle_player_by_id(&mut self, player_id: &PlayerId) -> Option<PlayerConnection> {
        return self.idle_players.remove(player_id);
    }
}

pub async fn lobby_manager_task(mut cmd_rcr: Receiver<LobbyCommand>, host_player: PlayerConnection) {

}