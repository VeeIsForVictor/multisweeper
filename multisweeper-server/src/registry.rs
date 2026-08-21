use std::collections::HashMap;

use anyhow::Result;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::{
    protocol::registry::RegistryMessage,
    room::{Room, RoomAddr, RoomCode},
};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("requested room with code {0} not found")]
    RoomNotFound(String),
    #[error("all senders dropped")]
    AddrDropped,
    #[error("reply failed")]
    ReplyFailed,
}

pub type RegistryMailbox = mpsc::Receiver<RegistryMessage>;
pub type RegistryAddr = mpsc::Sender<RegistryMessage>;

type ReplyHandle<T> = oneshot::Sender<T>;

pub enum RegistryEvent {
    Mailbox(Option<RegistryMessage>),
}

pub struct Registry {
    entity_counter: u64,
    rooms: HashMap<String, RoomAddr>,
    mailbox: RegistryMailbox,
    addr: RegistryAddr,
}

impl Registry {
    pub fn new() -> Self {
        let (addr, mailbox) = mpsc::channel::<RegistryMessage>(10);

        Registry {
            entity_counter: 0,
            rooms: HashMap::new(),
            mailbox,
            addr,
        }
    }

    fn generate_name(&mut self, prefix: &str) -> String {
        let id = self.entity_counter;
        self.entity_counter += 1;
        return format!("{prefix}{id:0>5}");
    }

    fn register_player(&mut self) -> String {
        self.generate_name("P")
    }

    async fn register_lobby(&mut self) -> (String, RoomAddr) {
        let code = self.generate_name("L");
        let room = Room::new(code.clone());
        let room_handle = &room.request_handle();
        tokio::spawn(room.handle_connection());
        self.rooms.insert(code.clone(), room_handle.clone());
        info!(
            target: "multisweeper.registry.room_created",
            room_code = %code,
            room_count = self.rooms.len(),
            "room created"
        );
        (code, room_handle.clone())
    }

    pub fn request_addr(&self) -> RegistryAddr {
        self.addr.clone()
    }

    fn request_lobby(&mut self, code: RoomCode) -> Result<RoomAddr, RegistryError> {
        match self.rooms.get(&code) {
            Some(handle) => Ok(handle.clone()),
            None => Err(RegistryError::RoomNotFound(code)),
        }
    }

    fn request_lobbies(&mut self) -> Vec<&RoomCode> {
        self.rooms.keys().collect()
    }

    #[tracing::instrument(name = "registry.lifecycle", skip_all)]
    pub async fn handle_connections(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
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
        let command = registry_message_name(&msg);
        debug!(
            target: "multisweeper.registry.command",
            command,
            "registry command received"
        );
        match msg {
            RegistryMessage::CreateLobby(reply) => {
                let (_code, addr) = self.register_lobby().await;
                Self::handle_reply(reply, addr).await;
                Ok(())
            }
            RegistryMessage::RequestLobby { code, reply } => {
                let result = self.request_lobby(code);
                if result.is_err() {
                    warn!(
                        target: "multisweeper.registry.room_lookup_failed",
                        error_type = "room_not_found",
                        "room lookup failed"
                    );
                }
                Self::handle_reply(reply, result).await;
                Ok(())
            }
            RegistryMessage::QueryLobbies(reply) => {
                let lobbies = self
                    .request_lobbies()
                    .iter()
                    .map(ToString::to_string)
                    .collect();
                Self::handle_reply(reply, lobbies).await;
                Ok(())
            }
            RegistryMessage::CreatePlayer(reply) => {
                let id = self.register_player();
                Self::handle_reply(reply, id).await;
                Ok(())
            }
        }
    }

    async fn handle_reply<T>(reply: ReplyHandle<T>, msg: T) -> () {
        let _ = reply.send(msg);
    }
}

fn registry_message_name(message: &RegistryMessage) -> &'static str {
    match message {
        RegistryMessage::CreateLobby(_) => "create_lobby",
        RegistryMessage::RequestLobby { .. } => "request_lobby",
        RegistryMessage::QueryLobbies(_) => "query_lobbies",
        RegistryMessage::CreatePlayer(_) => "create_player",
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
