mod action;
mod board;
mod error;
mod state;

use board::{Board, RevealResult};
use error::GameError;

use crate::{action::GameAction, state::{GameCell, GameSnapshot, GameStatus}};

type GameResult<T> = Result<T, GameError>;

pub struct GameInfo {
    pub width: u8,
    pub height: u8,
    pub number_of_mines: u8,
    pub seed: u64,
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
    last_state: Option<GameSnapshot>
}

impl Game {
    #[tracing::instrument]
    pub fn new(difficulty: GameDifficulty, seed: u64) -> Self {
        let board = Board::new(
            (difficulty as u8) * 4,
            (difficulty as u8) * 4,
            (difficulty as u8) * 3,
            seed,
        );
        Game {
            board: board.clone(),
            last_state: None,
            difficulty,
        }
    }

    pub fn info(&self) -> GameInfo {
        return GameInfo {
            width: self.board.width,
            height: self.board.height,
            number_of_mines: self.board.mines_count(),
            seed: self.board.seed,
        };
    }

    fn expose_board(&self) -> Vec<Vec<GameCell>> {
        return self.board.expose_cells()
    }

    fn set_state(&mut self, status: GameStatus) -> GameSnapshot {
        let new_state = GameSnapshot {
            status,
            board: self.expose_board()
        };
        self.last_state = Some(new_state.clone());
        return new_state.clone()
    }

    pub fn snapshot(&self) -> &Option<GameSnapshot> {
        return &self.last_state;
    }

    #[tracing::instrument(skip(self))]
    fn reveal(&mut self, x: u8, y: u8) -> GameResult<GameStatus> {
        let reveal_result = self.board.reveal(x, y)?;
        match reveal_result {
            RevealResult::Mine => Ok(GameStatus::Lost),
            RevealResult::DoNothing => Ok(GameStatus::Stalled),
            _ => Ok(GameStatus::Playing),
        }
    }

    #[tracing::instrument(skip(self))]
    fn flag(&mut self, x: u8, y: u8) -> Result<GameStatus, GameError> {
        match self.board.flag(x, y) {
            Ok(()) => Ok(GameStatus::Playing),
            Err(e) => Err(GameError::BoardError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn handle_action(&mut self, action: GameAction) -> Result<GameSnapshot, GameError> {
        match action {
            GameAction::Reveal { x, y } => {
                let reveal = self.reveal(x, y)?;
                if GameStatus::Playing == reveal
                    && self.board.is_all_safe_cells_revealed()
                {
                    return Ok(self.set_state(GameStatus::Won));
                }
                return Ok(self.set_state(reveal));
            }
            GameAction::Flag { x, y } => {
                let flag = self.flag(x, y)?;
                return Ok(self.set_state(flag));
            },
        }
    }

    pub fn lose_game(&mut self) {
        self.board.reveal_all();
    }
}