pub use crate::board::CellView as GameCell;

#[derive(Debug, Clone)]
pub struct GameSnapshot {
    pub status: GameStatus,
    pub action_result: GameActionResult,
    pub board: Vec<Vec<GameCell>>,
}

#[derive(Debug, Clone)]
pub enum GameStatus {
    Won,
    Lost,
    Playing,
}

impl From<ActionOutcome> for GameStatus {
    fn from(value: ActionOutcome) -> Self {
        use GameStatus::*;
        match value {
            ActionOutcome::Won => Won,
            ActionOutcome::Lost => Lost,
            ActionOutcome::Playing => Playing,
            ActionOutcome::Stalled => Playing,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameActionResult {
    Applied,
    Stalled,
    Started,
}

impl From<ActionOutcome> for GameActionResult {
    fn from(value: ActionOutcome) -> Self {
        use GameActionResult::*;
        match value {
            ActionOutcome::Won => Applied,
            ActionOutcome::Lost => Applied,
            ActionOutcome::Playing => Applied,
            ActionOutcome::Stalled => Stalled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionOutcome {
    Won,
    Lost,
    Playing,
    Stalled,
}
