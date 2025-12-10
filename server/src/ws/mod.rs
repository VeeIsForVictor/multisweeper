use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::ws::{lobby::LobbyCode, protocol::LobbyCommand};

mod lobby;
pub mod protocol;

pub type PlayerId = String;

pub struct SharedState {
    lobbies: HashMap<LobbyCode, mpsc::Sender<LobbyCommand>>
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            lobbies: HashMap::new()
        }
    }
}

pub fn lobby_manager_task(mut cmd_rcr: Receiver<LobbyCommand>) {

}