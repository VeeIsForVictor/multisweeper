#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("WebSocket connection was closed")]
    WebSocketClosed,

    #[error("Failed to deserialize client message: {0}")]
    MessageDeserializationFailed(String),

    #[error("Invalid state transition from {from} with action {action}")]
    StateTransitionInvalid { from: String, action: String },

    #[error("Lobby not found")]
    LobbyNotFound,

    #[error("Only the host can perform this action")]
    NotHost,

    #[error("Player not found")]
    PlayerNotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum LobbyError {
    #[error("Host disconnected from lobby")]
    HostDisconnected,

    #[error("No players remaining in lobby")]
    NoPlayersRemaining,
}
