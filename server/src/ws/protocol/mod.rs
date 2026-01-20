use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use crate::{game::GamePhase, ws::{PlayerId, lobby::LobbyCode}};
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
    StartGame,
    LeaveLobby
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum GameAction {
    RevealTile { x: u8, y: u8 },
    FlagTile { x: u8, y: u8 }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum GameResult {
    WON,
    LOST,
    PLAYING,
    STALLED,
}

impl From<GamePhase> for GameResult {
    fn from(value: GamePhase) -> Self {
        match value {
            GamePhase::WON => GameResult::WON,
            GamePhase::LOST => GameResult::LOST,
            GamePhase::PLAYING => GameResult::PLAYING,
            GamePhase::STALLED => GameResult::STALLED,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ErrorCode {
    DeserializationFailed,
    InvalidStateTransition,
    LobbyNotFound,
    NotHost,
    PlayerNotFound,
}

pub enum LobbyCommand {
    AddPlayer { id: PlayerId, player_connection: PlayerConnection, action_rcr: Receiver<ClientMessage> },
    RemovePlayer { id: PlayerId, return_to_idle: bool }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    LobbyState { code: LobbyCode, players: Vec<PlayerId>, host_id: PlayerId, status: LobbyStatus },
    GameStarted,
    GameInfo { code: LobbyCode, x_bound: u8, y_bound: u8, number_of_mines: u8, seed: u64 },
    GameAction(PlayerId, GameAction),
    GameResult(PlayerId, GameResult),
    Error { code: ErrorCode, message: String }
}