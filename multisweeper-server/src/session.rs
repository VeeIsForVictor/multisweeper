use tokio::sync::mpsc::{self, Sender, Receiver};

use crate::protocol::ServerMessage;

pub type PlayerId = String;
pub type PlayerHandle = Receiver<ServerMessage>;

pub struct Session {
    id: PlayerId,
    handle: PlayerHandle
}

impl Session {
    pub fn new(id: PlayerId) -> (Self, Sender<ServerMessage>) {
        let (sender, receiver) = mpsc::channel(10);
        return (Session { id, handle: receiver }, sender)
    }
}