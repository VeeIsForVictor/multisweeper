mod board;

use board::Board;

#[derive(Debug, Clone, Copy)]
pub enum GameDifficulty {
    EASY = 2,
    MEDIUM = 4,
    HARD = 5
}

#[derive(Debug)]
pub struct Game {
    board: Board,
    difficulty: GameDifficulty
} 

impl Game {
    pub fn new(difficulty: GameDifficulty) -> Self {
        Game {
            board: Board::new(
                (difficulty as u8) * 4,
                (difficulty as u8) * 4,
                (difficulty as u8) * 3
            ),
            difficulty
        }
    }
}

enum GamePhase {
    WON,
    LOST,
    PLAYING
}

pub struct GameState {
    phase: GamePhase,
}