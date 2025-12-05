mod board;
mod error;

use std::fmt::Display;

use board::Board;
use error::GameError;

use crate::game::board::RevealResult;

#[derive(Debug)]
pub enum GameAction {
    REVEAL { x: u8, y: u8 },
    FLAG { x: u8, y: u8 },
}

#[derive(Debug, Clone, Copy)]
pub enum GameDifficulty {
    TEST = 1,
    EASY = 2,
    MEDIUM = 4,
    HARD = 5,
}

#[derive(Debug)]
pub struct Game {
    board: Board,
    difficulty: GameDifficulty,
    state: GameState,
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
                (difficulty as u8) * 3,
            ),
            difficulty,
            state: GameState {
                phase: GamePhase::PLAYING,
            },
        }
    }

    fn is_coordinate_valid(&self, x: u8, y: u8) -> bool {
        self.board.is_coordinate_valid(x, y)
    }

    fn reveal(&mut self, x: u8, y: u8) -> Result<GamePhase, GameError> {
        if !self.is_coordinate_valid(x, y) {
            return Err(GameError);
        }

        let reveal_result = self.board.reveal(x, y);
        let Ok(revealed_state) = reveal_result else {
            return Err(GameError);
        };

        match revealed_state {
            RevealResult::Mine => Ok(GamePhase::LOST),
            RevealResult::DoNothing => Ok(GamePhase::STALLED),
            _ => Ok(GamePhase::PLAYING),
        }
    }

    fn flag(&mut self, x: u8, y: u8) -> Result<GamePhase, GameError> {
        match self.board.flag(x, y) {
            Ok(()) => Ok(GamePhase::PLAYING),
            Err(e) => Err(GameError),
        }
    }

    pub fn handle_action(&mut self, action: GameAction) -> Result<GamePhase, GameError> {
        match action {
            GameAction::REVEAL { x, y } => {
                let reveal = self.reveal(x, y);
                if let Ok(GamePhase::PLAYING) = reveal && self.board.is_all_safe_cells_revealed() {
                    return Ok(GamePhase::WON);
                }
                return reveal;
            },
            GameAction::FLAG { x, y } => self.flag(x, y),
        }
    }

    pub fn lose_game(&mut self) {
        self.board.reveal_all();
    }
}

#[derive(Debug)]
pub enum GamePhase {
    WON,
    LOST,
    PLAYING,
    STALLED,
}

#[derive(Debug)]
pub struct GameState {
    phase: GamePhase,
}
