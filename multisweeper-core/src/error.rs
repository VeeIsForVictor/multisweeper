use thiserror::Error;

use crate::{board::BoardError, state::GameSnapshot};

#[derive(Debug, Error)]
pub enum GameError {
    #[error("board error: {0}")]
    BoardError(#[from] BoardError),
    #[error("game already concluded with state")]
    ConcludedGame(GameSnapshot)
}
