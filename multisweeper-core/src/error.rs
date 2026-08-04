use thiserror::Error;

pub use crate::board::BoardError;
use crate::state::GameSnapshot;

#[derive(Debug, Clone, Error)]
pub enum GameError {
    #[error("board error: {0}")]
    BoardError(#[from] BoardError),
    #[error("game already concluded with state")]
    ConcludedGame(GameSnapshot),
}
