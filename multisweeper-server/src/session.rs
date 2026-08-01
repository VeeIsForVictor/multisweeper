use tokio::sync::mpsc::Receiver;

use crate::protocol::ServerMessage;

pub type PlayerId = String;
pub type PlayerHandle = Receiver<ServerMessage>;

pub struct Session {
    id: PlayerId,
    handle: PlayerHandle
}

impl Session {
    
}