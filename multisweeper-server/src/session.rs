use tokio::sync::mpsc::{self, Sender, Receiver};

use crate::protocol::ServerMessage;

pub type PlayerId = String;
pub type PlayerMailbox = Receiver<ServerMessage>;
pub type PlayerHandle = Sender<ServerMessage>;

pub struct Session {
    id: PlayerId,
    mailbox: PlayerMailbox,
    handle: PlayerHandle
}

impl Session {
    pub fn new(id: PlayerId) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        return Session { id, mailbox: receiver, handle: sender };
    }
}