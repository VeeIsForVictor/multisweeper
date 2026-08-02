use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{protocol::ClientMessage, registry::{RegistryHandle, RegistryOneShotSnd}, session::PlayerHandle};

pub type RoomCode = String;
pub type RoomMailbox = Receiver<ClientMessage>;
pub type RoomHandle = Sender<ClientMessage>;

pub enum RoomNotification {
    Empty
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    handle: RoomHandle,
    players: Vec<PlayerHandle>,
    owner: Option<PlayerHandle>
}

impl Room {
    pub fn new(code: RoomCode) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        return Room {
            code,
            mailbox: receiver,
            handle: sender,
            players: Vec::new(),
            owner: None
        };
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomHandle {
        self.handle.clone()
    }

    pub fn handle_connection(&mut self) -> Result<()> {
        loop {
            
        }
    }
}
