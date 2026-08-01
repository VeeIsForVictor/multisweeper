pub mod action;
mod board;
pub mod error;
pub mod state;

use board::{Board, RevealResult};
use error::GameError;

use crate::{
    action::GameAction,
    state::{ActionOutcome, GameActionResult, GameCell, GameSnapshot, GameStatus},
};

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
    pub difficulty: GameDifficulty,
    board: Board,
    last_state: GameSnapshot,
}

impl Game {
    #[tracing::instrument]
    pub fn new(difficulty: GameDifficulty, seed: u64) -> GameResult<Self> {
        let board = Board::new(
            (difficulty as u8) * 4,
            (difficulty as u8) * 4,
            (difficulty as u8) * 3,
            seed,
        )?;
        Ok(Game {
            board: board.clone(),
            last_state: GameSnapshot {
                status: GameStatus::Playing,
                action_result: GameActionResult::Started,
                board: board.expose_cells(),
            },
            difficulty,
        })
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
        return self.board.expose_cells();
    }

    fn set_state(&mut self, outcome: ActionOutcome) -> &GameSnapshot {
        let new_state = GameSnapshot {
            status: outcome.into(),
            action_result: outcome.into(),
            board: self.expose_board(),
        };
        self.last_state = new_state.clone();
        return &self.last_state;
    }

    pub fn snapshot(&self) -> &GameSnapshot {
        return &self.last_state;
    }

    #[tracing::instrument(skip(self))]
    fn reveal(&mut self, x: u8, y: u8) -> GameResult<ActionOutcome> {
        let reveal_result = self.board.reveal(x, y)?;
        match reveal_result {
            RevealResult::Mine => Ok(ActionOutcome::Lost),
            RevealResult::DoNothing => Ok(ActionOutcome::Stalled),
            _ => Ok(ActionOutcome::Playing),
        }
    }

    #[tracing::instrument(skip(self))]
    fn flag(&mut self, x: u8, y: u8) -> Result<ActionOutcome, GameError> {
        match self.board.flag(x, y) {
            Ok(()) => Ok(ActionOutcome::Playing),
            Err(e) => Err(GameError::BoardError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn handle_action(&mut self, action: GameAction) -> Result<&GameSnapshot, GameError> {
        match self.last_state.status {
            GameStatus::Playing => {}
            GameStatus::NoWinner | GameStatus::Won => {
                return Err(GameError::ConcludedGame(self.last_state.clone()));
            }
        }
        match action {
            GameAction::Reveal { x, y } => {
                let reveal = self.reveal(x, y)?;
                if ActionOutcome::Playing == reveal && self.board.is_all_safe_cells_revealed() {
                    return Ok(self.set_state(ActionOutcome::Won));
                }
                return Ok(self.set_state(reveal));
            }
            GameAction::Flag { x, y } => {
                let flag = self.flag(x, y)?;
                return Ok(self.set_state(flag));
            }
        }
    }

    pub fn lose_game(&mut self) {
        self.board.reveal_all();
        self.last_state = GameSnapshot {
            status: GameStatus::NoWinner,
            action_result: GameActionResult::Eliminated,
            board: self.expose_board(),
        }
    }
}
