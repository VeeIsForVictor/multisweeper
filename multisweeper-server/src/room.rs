use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{protocol::room::RoomMessage, session::PlayerAddr};

pub type RoomCode = String;
pub type RoomMailbox = Receiver<RoomMessage>;
pub type RoomAddr = Sender<RoomMessage>;

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: Vec<PlayerAddr>,
    owner: Option<PlayerAddr>
}

impl Room {
    pub fn new(code: RoomCode) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        return Room {
            code,
            mailbox: receiver,
            addr: sender,
            players: Vec::new(),
            owner: None
        };
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomAddr {
        self.addr.clone()
    }

    pub async fn handle_connection(self) -> Result<()> {
        loop {
            
        }
    }
}
