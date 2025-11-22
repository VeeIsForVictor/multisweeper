mod board;

use std::fmt::Display;

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

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "{0:?}\n", self.difficulty);
        return write!(f, "{0}", self.board);
    }
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