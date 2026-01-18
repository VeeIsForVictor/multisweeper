use crate::ws::protocol::ErrorCode;

#[derive(Debug, Clone, thiserror::Error)]
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum LobbyError {
    #[error("Host disconnected from lobby")]
    HostDisconnected,

    #[error("No players remaining in lobby")]
    NoPlayersRemaining,
}

impl From<ConnectionError> for ErrorCode {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::WebSocketClosed => ErrorCode::DeserializationFailed,
            ConnectionError::MessageDeserializationFailed(_) => ErrorCode::DeserializationFailed,
            ConnectionError::StateTransitionInvalid { .. } => ErrorCode::InvalidStateTransition,
            ConnectionError::LobbyNotFound => ErrorCode::LobbyNotFound,
            ConnectionError::NotHost => ErrorCode::NotHost,
            ConnectionError::PlayerNotFound => ErrorCode::PlayerNotFound,
        }
    }
}

impl From<LobbyError> for ErrorCode {
    fn from(err: LobbyError) -> Self {
        match err {
            LobbyError::HostDisconnected => ErrorCode::InvalidStateTransition,
            LobbyError::NoPlayersRemaining => ErrorCode::LobbyNotFound,
        }
    }
}
