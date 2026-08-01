#[derive(Debug, Clone)]
pub enum GameStatus {
    WON,
    LOST,
    PLAYING(String),
    STALLED,
}

#[derive(Debug, Clone)]
pub struct GameState {
    status: GameStatus,
}
