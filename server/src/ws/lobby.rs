use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

pub enum LobbyCommand {
    AddPlayer { id: String,  }
}