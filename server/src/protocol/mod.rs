use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Ping{ time: u128 }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Pong{ ping_time: u128, pong_time: u128 },
    Error(String)
}