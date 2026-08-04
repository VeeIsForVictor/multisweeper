use std::{collections::HashMap, todo};

use anyhow::Result;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{protocol::{room::{PlayerCommand, RoomMessage}, session::SessionMessage}, session::{PlayerAddr, PlayerId}};

pub type RoomCode = String;
pub type RoomMailbox = Receiver<RoomMessage>;
pub type RoomAddr = Sender<RoomMessage>;

#[derive(Debug, Error)]
pub enum RoomError {
    #[error("mailbox dropped")]
    MailboxDropped,
    #[error("no owner registered")]
    NoOwner,
    #[error("player dropped {0}")]
    PlayerDropped(PlayerId),
    #[error("no such player {0}")]
    NoPlayerFound(PlayerId),
    #[error("no players remaining")]
    AllPlayersDropped
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: HashMap<PlayerId, PlayerAddr>,
    owner: Option<PlayerId>,
}

pub struct RoomState {
    pub code: RoomCode,
    pub players: Vec<PlayerId>,
    pub owner: PlayerId
}

enum RoomEvent {
    Session(Option<RoomMessage>)
}

impl Room {
    pub fn new(code: RoomCode) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        return Room {
            code,
            mailbox: receiver,
            addr: sender,
            players: HashMap::new(),
            owner: None,
        };
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomAddr {
        self.addr.clone()
    }

    pub fn state(&self) -> Result<RoomState, RoomError> {
        return Ok(RoomState { 
            code: self.code().to_string(), 
            players: self.players.keys().map(ToString::to_string).collect(), 
            owner: match &self.owner {
                Some(id) => id.clone(),
                None => return Err(RoomError::NoOwner)
            } 
        })
    }

    fn register_player(&mut self, id: PlayerId, addr: PlayerAddr ) {
        if let None = self.owner {
            self.owner = Some(id.clone());
        }
        self.players.insert(id, addr);
    }

    pub async fn handle_connection(mut self) -> Result<()> {
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
                msg = self.mailbox.recv() => RoomEvent::Session(msg)
            };

            match event {
                RoomEvent::Session(msg) => {
                    let msg = self.receive_mailbox(msg)?;
                    self.handle_mailbox(msg).await?
                }
            }
        }
    }

    fn receive_mailbox(&self, msg: Option<RoomMessage>) -> Result<RoomMessage> {
        match msg {
            Some(msg) => Ok(msg),
            None => Err(RoomError::MailboxDropped.into()),
        }
    }

    async fn handle_mailbox(&mut self, msg: RoomMessage) -> Result<()> {
        let player_id = msg.id;
        match msg.command {
            PlayerCommand::Join { handle } => {
                self.register_player(player_id, handle.clone());
                Ok(self.broadcast_state().await?)
            },
            PlayerCommand::Leave => todo!(),
        }
    }

    async fn register_new_owner(&mut self) -> Result<PlayerId, RoomError> {
        let new_owner = match self.players.keys().next() {
            Some(id) => id.clone(),
            None => return Err(RoomError::AllPlayersDropped)
        };

        self.owner = Some(new_owner.to_owned());
        return Ok(new_owner);
    }

    async fn drop_player(&mut self, id: &PlayerId) -> Result<(), RoomError> {
        if Some(id.clone()) == self.owner {
            self.owner = None;
            self.register_new_owner().await?;
        }
        self.send_player(id, SessionMessage::Kicked).await?;
        let addr = match self.players.remove(id) {
            Some(addr) => addr,
            None => return Ok(()),
        };
        Ok(self.broadcast_state().await?)
    }

    async fn broadcast_state(&mut self) -> Result<(), RoomError> {
        let state = match self.state() {
            Ok(state) => state,
            Err(err) => return Err(err)
        };

        return match self.broadcast_message(state.into()).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        };
    }

    async fn broadcast_message(&mut self, msg: SessionMessage) -> Result<(), RoomError> {
        let ids: Vec<PlayerId> = self.players.keys().map(ToString::to_string).collect();
        for id in ids {
            self.send_player(&id, msg.clone()).await?;
        };
        return Ok(())
    }

    async fn send_player(&mut self, id: &PlayerId, msg: SessionMessage) -> Result<(), RoomError> {
        let addr: &PlayerAddr = match self.players.get_mut(id) {
            Some(addr) => addr,
            None => return Err(RoomError::NoPlayerFound(id.clone()).into()),
        };
        
        return match addr.send(msg).await {
            Ok(()) => Ok(()),
            Err(e) => Err(RoomError::PlayerDropped(id.clone()).into())
        }
    }
}