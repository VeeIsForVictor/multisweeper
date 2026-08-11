use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum GameAction {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
}
