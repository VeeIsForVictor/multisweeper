use schemars::JsonSchema;
use serde::Serialize;

pub use crate::board::CellView as GameCell;

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub struct GameSnapshot {
    pub status: GameStatus,
    pub action_result: GameActionResult,
    pub board: Vec<Vec<GameCell>>,
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub enum GameStatus {
    Won,
    NoWinner,
    Playing,
}

impl From<ActionOutcome> for GameStatus {
    fn from(value: ActionOutcome) -> Self {
        use GameStatus::*;
        match value {
            ActionOutcome::Won => Won,
            ActionOutcome::Lost => Playing,
            ActionOutcome::Playing => Playing,
            ActionOutcome::Stalled => Playing,
        }
    }
}

#[derive(Serialize, Debug, Clone, JsonSchema)]
pub enum GameActionResult {
    Applied,
    Stalled,
    Started,
    Eliminated,
    Won,
}

impl From<ActionOutcome> for GameActionResult {
    fn from(value: ActionOutcome) -> Self {
        use GameActionResult::*;
        match value {
            ActionOutcome::Won => Won,
            ActionOutcome::Lost => Eliminated,
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
