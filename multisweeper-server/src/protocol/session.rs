use asyncapi_rust::schemars::JsonSchema;
use multisweeper_core::GameSnapshot;
use serde::Serialize;

use crate::{
    room::{RoomCode, RoomState},
    session::PlayerId,
};

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

#[derive(Debug, Serialize, Clone, JsonSchema)]
pub enum SessionMessage {
    RoomState {
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<PlayerView>,
        game: MatchView,
    },
    Kicked {
        reason: String,
    },
    Error {
        reason: String,
    },
    GameStarted,
}

impl From<RoomState> for SessionMessage {
    fn from(value: RoomState) -> Self {
        SessionMessage::RoomState {
            code: value.code,
            owner: value.owner,
            players: value.players,
            game: value.match_state,
        }
    }
}
