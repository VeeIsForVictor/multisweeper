use tokio::sync::mpsc::Receiver;

use crate::protocol::ClientMessage;

pub type RoomCode = String;
pub type RoomHandle = Receiver<ClientMessage>;

pub struct Room {
    code: RoomCode,
    handle: RoomHandle
}

impl Room {
    
}