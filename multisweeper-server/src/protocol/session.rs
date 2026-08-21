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
    RoomState {
        correlation_id: Option<MessageId>,
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerView>,
        game: MatchView,
    },
    RoomRemoved {
        correlation_id: Option<MessageId>,
        reason: String,
    },
    RoomJoinRejected {
        correlation_id: Option<MessageId>,
        error: ClientError,
    },
    Error {
        correlation_id: Option<MessageId>,
        error: ClientError,
    },
    GameStarted {
        correlation_id: Option<MessageId>,
    },
}

impl SessionMessage {
    pub fn with_correlation_id(mut self, correlation_id: Option<MessageId>) -> Self {
        match &mut self {
            Self::RoomState {
                correlation_id: current,
                ..
            }
            | Self::RoomRemoved {
                correlation_id: current,
                ..
            }
            | Self::RoomJoinRejected {
                correlation_id: current,
                ..
            }
            | Self::Error {
                correlation_id: current,
                ..
            }
            | Self::GameStarted {
                correlation_id: current,
            } => *current = correlation_id,
        }
        self
    }
}

impl From<RoomState> for SessionMessage {
    fn from(value: RoomState) -> Self {
        SessionMessage::RoomState {
            correlation_id: None,
            code: value.code,
            owner: value.owner,
            players: value.players,
            game: value.match_state,
        }
    }
}
