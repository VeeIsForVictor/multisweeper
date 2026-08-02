use std::collections::HashMap;

use anyhow::Result;
use parking_lot::Mutex;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use triomphe::Arc;

use crate::{protocol::RegistryMessage, room::{Room, RoomAddr, RoomCode}};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("requested room with code {0} not found")]
    RoomNotFound(String),
    #[error("all senders dropped")]
    AddrDropped,
    #[error("reply failed")]
    ReplyFailed
}

pub type RegistryLock = Arc<Mutex<Registry>>;
pub type RegistryMailbox = mpsc::Receiver<RegistryMessage>;
pub type RegistryAddr = mpsc::Sender<RegistryMessage>;

type ReplyHandle<T> = oneshot::Sender<T>;

pub enum RegistryEvent {
    Mailbox(Option<RegistryMessage>)
}

pub struct Registry {
    entity_counter: u64,
    rooms: HashMap<String, RoomAddr>,
    mailbox: RegistryMailbox,
    addr: RegistryAddr
}

impl Registry {
    pub fn new() -> Self {
        let (addr, mailbox) = mpsc::channel::<RegistryMessage>(10);

        return Registry {
            entity_counter: 0,
            rooms: HashMap::new(),
            mailbox,
            addr
        };
    }

    fn generate_name(&mut self, prefix: &str) -> String {
        let id = self.entity_counter;
        self.entity_counter += 1;
        String::from(format!("{prefix}{id:0>5}"))
    }

    fn register_player(&mut self) -> String {
        self.generate_name("P")
    }

    fn register_lobby(&mut self) -> (String, RoomAddr) {
        let code = self.generate_name("L");
        let room = Room::new(code.clone());
        self.rooms.insert(code.clone(), room.request_handle());
        return (code, room.request_handle());
    }

    pub fn request_addr(&self) -> RegistryAddr {
        self.addr.clone()
    }

    fn request_lobby(&mut self, code: RoomCode) -> Result<RoomAddr, RegistryError> {
        match self.rooms.get(&code) {
            Some(handle) => Ok(handle.clone()),
            None => Err(RegistryError::RoomNotFound(code).into())
        }
    }

    fn request_lobbies(&mut self) -> Vec<&RoomCode> {
        self.rooms.keys().collect()
    }

    pub async fn handle_connections(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => Ok(()),
            Err(e) => {
                Err(e)
            }
        }
    }

    async fn event_loop(&mut self) -> Result<()> {
        loop {
            let event = tokio::select! { 
                msg = self.mailbox.recv() => RegistryEvent::Mailbox(msg)
            };

            match event {
                RegistryEvent::Mailbox(msg) => {
                    let msg = self.receive_mailbox(msg)?;
                    self.handle_mailbox(msg).await?;
                }
            }
        }
    }

    fn receive_mailbox(&mut self, msg: Option<RegistryMessage>) -> Result<RegistryMessage> {
        match msg {
            Some(msg) => Ok(msg),
            None => Err(RegistryError::AddrDropped.into()),
        }
    }

    async fn handle_mailbox(&mut self, msg: RegistryMessage) -> Result<()> {
        match msg {
            RegistryMessage::CreateLobby(reply) => {
                let (_code, addr) = self.register_lobby();
                return Ok(Self::handle_reply(reply, addr).await);
            },
            RegistryMessage::RequestLobby{code, reply} => {
                let result = self.request_lobby(code);
                return Ok(Self::handle_reply(reply, result).await);
            },
            RegistryMessage::QueryLobbies(reply) => {
                let lobbies = self.request_lobbies().iter().map(ToString::to_string).collect();
                return Ok(Self::handle_reply(reply, lobbies).await);
            },
        }
    }

    async fn handle_reply<T>(reply: ReplyHandle<T>, msg: T) -> () {
        reply.send(msg);
    }
}
