use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::protocol::ClientMessage;

pub type RoomCode = String;
pub type RoomMailbox = Receiver<ClientMessage>;
pub type RoomHandle = Sender<ClientMessage>;

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    handle: RoomHandle,
}

impl Room {
    pub fn new(code: RoomCode) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        return Room {
            code,
            mailbox: receiver,
            handle: sender,
        };
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomHandle {
        self.handle.clone()
    }
}
