use asyncapi_rust::{ToAsyncApiMessage, schemars::JsonSchema};
use multisweeper_core::{GameAction, GameDifficulty};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    protocol::session::{ClientError, MessageId, SessionEvent, SessionMessage},
    room::RoomCode,
    session::PlayerId,
};

#[derive(Debug, Deserialize, Serialize, JsonSchema, ToAsyncApiMessage)]
#[serde(tag = "type")]
pub enum ClientRequest {
    #[serde(rename = "connection.ping")]
    ConnectionPing { message_id: MessageId },
    #[serde(rename = "rooms.list")]
    RoomsList { message_id: MessageId },
    #[serde(rename = "room.join")]
    RoomJoin {
        message_id: MessageId,
        room_code: RoomCode,
    },
    #[serde(rename = "room.create")]
    RoomCreate { message_id: MessageId },
    #[serde(rename = "room.leave")]
    RoomLeave { message_id: MessageId },
    #[serde(rename = "game.start")]
    GameStart {
        message_id: MessageId,
        difficulty: ClientDifficulty,
    },
    #[serde(rename = "game.action")]
    GameAction {
        message_id: MessageId,
        action: ClientGameAction,
        x: u8,
        y: u8,
    },
    #[serde(rename = "room.state.get")]
    RoomStateGet { message_id: MessageId },
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientGameAction {
    Reveal,
    Flag,
}

impl ClientGameAction {
    pub fn into_game_action(self, x: u8, y: u8) -> GameAction {
        match self {
            ClientGameAction::Reveal => GameAction::Reveal { x, y },
            ClientGameAction::Flag => GameAction::Flag { x, y },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
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
        serde_json::from_slice(&value.into_data())
    }
}

#[derive(Debug, Serialize, JsonSchema, ToAsyncApiMessage)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "connection.ready")]
    ConnectionReady {
        message_id: MessageId,
        player_id: PlayerId,
    },
    #[serde(rename = "connection.pong")]
    ConnectionPong {
        message_id: MessageId,
        correlation_id: MessageId,
    },
    #[serde(rename = "rooms.listed")]
    RoomsListed {
        message_id: MessageId,
        correlation_id: MessageId,
        rooms: Vec<RoomCode>,
    },
    #[serde(rename = "room.state")]
    RoomState {
        message_id: MessageId,
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<MessageId>,
        code: RoomCode,
        owner: Option<PlayerId>,
        players: Vec<crate::protocol::session::PlayerView>,
        game: crate::protocol::session::MatchView,
    },
    #[serde(rename = "room.removed")]
    RoomRemoved {
        message_id: MessageId,
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<MessageId>,
        reason: String,
    },
    #[serde(rename = "command.rejected")]
    CommandRejected {
        message_id: MessageId,
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<MessageId>,
        error: ClientError,
    },
    #[serde(rename = "game.started")]
    GameStarted {
        message_id: MessageId,
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<MessageId>,
    },
}

impl ServerMessage {
    pub fn from_session(message_id: MessageId, value: SessionMessage) -> Self {
        let (correlation_id, value) = match value {
            SessionMessage::Reply {
                request_id,
                message,
            } => (Some(request_id), message),
            SessionMessage::Broadcast(message) => (None, message),
        };
        match value {
            SessionEvent::RoomState {
                code,
                owner,
                players,
                game,
            } => Self::RoomState {
                message_id,
                correlation_id,
                code,
                owner,
                players,
                game,
            },
            SessionEvent::RoomRemoved { reason } => Self::RoomRemoved {
                message_id,
                correlation_id,
                reason,
            },
            SessionEvent::RoomJoinRejected { error } | SessionEvent::Error { error } => {
                Self::CommandRejected {
                    message_id,
                    correlation_id,
                    error,
                }
            }
            SessionEvent::GameStarted => Self::GameStarted {
                message_id,
                correlation_id,
            },
        }
    }
}

impl TryInto<Message> for ServerMessage {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<Message, Self::Error> {
        let response = serde_json::to_string::<ServerMessage>(&self)?;
        Ok(Message::Text(response.into()))
    }
}

impl ClientRequest {
    pub fn message_id(&self) -> &MessageId {
        match self {
            Self::ConnectionPing { message_id }
            | Self::RoomsList { message_id }
            | Self::RoomCreate { message_id }
            | Self::RoomLeave { message_id }
            | Self::GameStart { message_id, .. }
            | Self::GameAction { message_id, .. }
            | Self::RoomStateGet { message_id }
            | Self::RoomJoin { message_id, .. } => message_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::session::{SessionEvent, SessionMessage};

    use super::{ClientGameAction, ClientRequest, ServerMessage};

    #[test]
    fn client_messages_are_flat_and_correlatable() {
        let request = ClientRequest::RoomJoin {
            message_id: "req-1".to_string(),
            room_code: "L00001".to_string(),
        };

        let json = serde_json::to_value(request).expect("request should serialize");
        assert_eq!(json["type"], "room.join");
        assert_eq!(json["message_id"], "req-1");
        assert_eq!(json["room_code"], "L00001");
        assert!(json.get("payload").is_none());
    }

    #[test]
    fn server_messages_are_flat_and_preserve_correlation() {
        let response = ServerMessage::GameStarted {
            message_id: "m-1".to_string(),
            correlation_id: Some("req-1".to_string()),
        };

        let json = serde_json::to_value(response).expect("response should serialize");
        assert_eq!(json["type"], "game.started");
        assert_eq!(json["message_id"], "m-1");
        assert_eq!(json["correlation_id"], "req-1");
        assert!(json.get("payload").is_none());
    }

    #[test]
    fn game_actions_use_the_same_type_discriminator() {
        let request = ClientRequest::GameAction {
            message_id: "req-1".to_string(),
            action: ClientGameAction::Reveal,
            x: 2,
            y: 3,
        };
        let json = serde_json::to_value(request).expect("action should serialize");

        assert_eq!(json["type"], "game.action");
        assert_eq!(json["action"], "reveal");
        assert_eq!(json["x"], 2);
        assert_eq!(json["y"], 3);
        assert!(json.get("payload").is_none());
    }

    #[test]
    fn replies_preserve_request_ids_but_broadcasts_are_uncorrelated() {
        let reply = ServerMessage::from_session(
            "m-1".to_string(),
            SessionMessage::Reply {
                request_id: "req-1".to_string(),
                message: SessionEvent::GameStarted,
            },
        );
        let broadcast = ServerMessage::from_session(
            "m-2".to_string(),
            SessionMessage::Broadcast(SessionEvent::GameStarted),
        );

        let reply_json = serde_json::to_value(reply).expect("reply should serialize");
        let broadcast_json = serde_json::to_value(broadcast).expect("broadcast should serialize");

        assert_eq!(reply_json["correlation_id"], "req-1");
        assert!(broadcast_json.get("correlation_id").is_none());
    }
}
