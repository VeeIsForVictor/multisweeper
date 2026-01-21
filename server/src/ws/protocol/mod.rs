use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use crate::{game::{GameAction, GamePhase}, ws::{PlayerId, lobby::LobbyCode}};
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
    GameClient (PlayerAction)
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
pub enum PlayerAction {
    RevealTile { x: u8, y: u8 },
    FlagTile { x: u8, y: u8 }
}

impl Into<crate::game::GameAction> for PlayerAction {
    fn into(self) -> crate::game::GameAction {
        match self {
            PlayerAction::RevealTile { x, y } => crate::game::GameAction::REVEAL { x, y },
            PlayerAction::FlagTile { x, y } => crate::game::GameAction::FLAG { x, y },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PlayerResult {
    WON,
    LOST,
    PLAYING(String),
    STALLED,
    TIMEOUT
}

impl From<GamePhase> for PlayerResult {
    fn from(value: GamePhase) -> Self {
        match value {
            GamePhase::WON => PlayerResult::WON,
            GamePhase::LOST => PlayerResult::LOST,
            GamePhase::PLAYING(board_string) => PlayerResult::PLAYING(board_string),
            GamePhase::STALLED => PlayerResult::STALLED,
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
    NotYourTurn,
    GameLogicError
}

pub enum LobbyCommand {
    AddPlayer { id: PlayerId, player_connection: PlayerConnection, action_rcr: Receiver<ClientMessage> },
    RemovePlayer { id: PlayerId, return_to_idle: bool }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    LobbyState { code: LobbyCode, players: Vec<PlayerId>, host_id: PlayerId, status: LobbyStatus },
    GameStarted,
    GameInfo { code: LobbyCode, width: u8, height: u8, number_of_mines: u8, seed: u64 },
    PlayerTurn(PlayerId),
    PlayerAction(PlayerId, PlayerAction),
    PlayerResult(PlayerId, PlayerResult),
    Error { code: ErrorCode, message: String }
}