use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::ws::{lobby::LobbyCode, protocol::{ClientMessage, LobbyCommand, PlayerConnection, ServerMessage}};

mod lobby;
pub mod protocol;

pub type PlayerId = String;

pub struct SharedState {
    lobbies: HashMap<LobbyCode, mpsc::Sender<LobbyCommand>>,
    players: HashMap<PlayerId, PlayerConnection>
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            lobbies: HashMap::new(),
            players: HashMap::new()
        }
    }

    pub fn register_player(&mut self, mut action_sdr: Sender<ClientMessage>, mut message_sdr: Sender<ServerMessage>) {
        let connection = PlayerConnection { action_sdr: action_sdr, message_sdr: message_sdr };
    }
}

pub fn lobby_manager_task(mut cmd_rcr: Receiver<LobbyCommand>, host_player: PlayerConnection) {

}