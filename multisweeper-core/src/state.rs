pub use crate::board::CellView as GameCell;

#[derive(Debug, Clone)]
pub struct GameSnapshot {
    pub status: GameStatus,
    pub board: Vec<Vec<GameCell>>
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameStatus {
    Won,
    Lost,
    Playing,
    Stalled,
}