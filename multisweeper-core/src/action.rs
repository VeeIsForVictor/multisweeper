#[derive(Debug)]
pub enum GameAction {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
}
