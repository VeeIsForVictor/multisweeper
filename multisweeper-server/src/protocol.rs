use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

use crate::room::RoomCode;

pub enum ClientMessage {}

pub enum ServerMessage {}

#[derive(Deserialize)]
#[serde(tag = "type", content = "json")]
pub enum ClientRequest {
    Ping,
    JoinRoom { room_code: RoomCode },
    LeaveRoom,
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

#[derive(Serialize)]
#[serde(tag = "type", content = "json")]
pub enum ServerResponse {
    Pong,
    AdvertiseRooms { rooms: Vec<RoomCode> },
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
