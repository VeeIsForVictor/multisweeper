use thiserror::Error;

use crate::board::BoardError;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("board error: {0}")]
    BoardError(#[from] BoardError),
}
