mod board;

use board::Board;

#[derive(Debug)]
pub struct Game {
    board: Board,
}

enum GamePhase {
    WON,
    LOST,
    PLAYING
}

pub struct GameState {
    phase: GamePhase,
}