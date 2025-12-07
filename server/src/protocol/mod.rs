use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Ping{ time: u64 }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Pong{ ping_time: u64, pong_time: u64 }
}