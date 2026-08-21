use asyncapi_rust::schemars::JsonSchema;
use multisweeper_core::GameSnapshot;
use serde::Serialize;

use crate::{
    room::{RoomCode, RoomState},
    session::PlayerId,
};

pub type MessageId = String;

#[derive(Debug, Serialize, Clone, JsonSchema, PartialEq, Eq)]
pub enum PlayerState {
    Spectator,
    Playing,
    Eliminated,
}

#[derive(Debug, Serialize, Clone, JsonSchema)]
pub struct PlayerView {
    pub id: PlayerId,
    pub state: PlayerState,
}

#[derive(Debug, Serialize, Clone, JsonSchema, PartialEq, Eq)]
pub enum MatchState {
    Waiting,
    Playing {
        last_player: Option<PlayerId>,
        current_player: PlayerId,
    },
    Won,
    NoWinner,
}

#[derive(Debug, Serialize, Clone, JsonSchema)]
pub struct MatchView {
    pub state: MatchState,
    pub game: Option<GameSnapshot>,
}

#[derive(Debug, Serialize, Clone, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    RoomAlreadyJoined,
    NoRoomJoined,
    RoomDropped,
    RoomNotFound,
    NotRoomOwner,
    GameAlreadyStarted,
    GameNotStarted,
    GameEnded,
    PlayerIsSpectating,
    PlayerEliminated,
    NotCurrentPlayer,
    NoPlayersRemaining,
    PlayerNotFound,
    GameError,
    RoomUnavailable,
    InvalidMessage,
    DuplicateMessageId,
}

#[derive(Debug, Serialize, Clone, JsonSchema, PartialEq, Eq)]
pub struct ClientError {
    pub code: ErrorCode,
    pub message: String,
}

impl ClientError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionMessage {
    Reply {
        request_id: MessageId,
        message: SessionEvent,
    },
    Broadcast(SessionEvent),
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    RoomState {
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerView>,
        game: MatchView,
    },
    RoomRemoved {
        reason: String,
    },
    RoomJoinRejected {
        error: ClientError,
    },
    Error {
        error: ClientError,
    },
    GameStarted,
}

impl From<RoomState> for SessionEvent {
    fn from(value: RoomState) -> Self {
        SessionEvent::RoomState {
            code: value.code,
            owner: value.owner,
            players: value.players,
            game: value.match_state,
        }
    }
}
