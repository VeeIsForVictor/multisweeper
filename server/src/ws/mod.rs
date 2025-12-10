use std::collections::HashMap;
use tokio::sync::mpsc;

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