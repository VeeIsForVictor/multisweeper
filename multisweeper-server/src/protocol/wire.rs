use asyncapi_rust::{ToAsyncApiMessage, schemars::JsonSchema};
use multisweeper_core::{GameAction, GameDifficulty};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

use crate::{protocol::session::SessionMessage, room::RoomCode};

#[derive(Deserialize, Serialize, JsonSchema, ToAsyncApiMessage)]
#[serde(tag = "type", content = "payload")]
pub enum ClientRequest {
    Ping,
    // Room-related
    QueryRooms,
    JoinRoom { room_code: RoomCode },
    CreateRoom,
    LeaveRoom,
    // Game-related
    StartGame { difficulty: ClientDifficulty },
    GameAction { action: ClientGameAction },
    GameQuery,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "action")]
pub enum ClientGameAction {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
}

impl From<ClientGameAction> for GameAction {
    fn from(value: ClientGameAction) -> Self {
        match value {
            ClientGameAction::Reveal { x, y } => GameAction::Reveal { x, y },
            ClientGameAction::Flag { x, y } => GameAction::Flag { x, y },
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub enum ClientDifficulty {
    Test,
    Easy,
    Medium,
    Hard,
}

impl From<ClientDifficulty> for GameDifficulty {
    fn from(value: ClientDifficulty) -> Self {
        match value {
            ClientDifficulty::Test => GameDifficulty::TEST,
            ClientDifficulty::Easy => GameDifficulty::EASY,
            ClientDifficulty::Medium => GameDifficulty::MEDIUM,
            ClientDifficulty::Hard => GameDifficulty::HARD,
        }
    }
}

impl TryFrom<Message> for ClientRequest {
    type Error = serde_json::Error;
    fn try_from(value: Message) -> Result<Self, Self::Error> {
        let bytes = value.into_data();
        match serde_json::from_slice::<ClientRequest>(&bytes) {
            Ok(req) => Ok(req),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Serialize, JsonSchema, ToAsyncApiMessage)]
#[serde(tag = "type", content = "payload")]
pub enum ServerResponse {
    Pong,
    AdvertiseRooms { rooms: Vec<RoomCode> },
    ClientError(String),
    Message(SessionMessage),
}

impl TryInto<Message> for ServerResponse {
    type Error = serde_json::Error;
    fn try_into(self) -> Result<Message, Self::Error> {
        match serde_json::to_string::<ServerResponse>(&self) {
            Ok(res) => Ok(Message::Text(res.into())),
            Err(e) => Err(e),
        }
    }
}
