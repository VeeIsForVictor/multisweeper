use tokio::sync::mpsc::Receiver;

use crate::ws::{lobby::LobbyCode, protocol::ClientMessage};

pub enum PlayerStatus {
    Idle { action_rcr: Receiver<ClientMessage> },
    Lobby { code: LobbyCode },
    Game
}