use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use crate::ws::{PlayerId, lobby::LobbyCode};
use super::lobby::LobbyStatus;

#[derive(Debug)]
pub struct PlayerConnection {
    pub action_sdr: Sender<ClientMessage>,
    pub message_sdr: Sender<ServerMessage>
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    IdleClient (IdleAction),
    LobbyClient (LobbyAction),
    GameClient (GameAction)
}

#[derive(Deserialize, Serialize, Debug)]
pub enum IdleAction {
    CreateLobby,
    JoinLobby { code: LobbyCode }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum LobbyAction {
    StartGame
}

#[derive(Deserialize, Serialize, Debug)]
pub enum GameAction {

}

pub enum LobbyCommand {
    AddPlayer { id: PlayerId, player_connection: PlayerConnection, action_rcr: Receiver<ClientMessage> },
    RemovePlayer(PlayerId)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    LobbyState { code: LobbyCode, players: Vec<PlayerId>, host_id: PlayerId, status: LobbyStatus },
    GameStarted,
    Error(String)
}