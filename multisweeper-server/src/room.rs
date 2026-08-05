use std::{collections::HashMap, println, todo};

use anyhow::Result;
use multisweeper_core::{Game, GameDifficulty, GameError, GameSnapshot};
use rand::random;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{protocol::{room::{PlayerCommand, RoomMessage}, session::SessionMessage}, session::{PlayerAddr, PlayerId}};

pub type RoomCode = String;
pub type RoomMailbox = Receiver<RoomMessage>;
pub type RoomAddr = Sender<RoomMessage>;

#[derive(Debug, Clone, Error)]
pub enum RoomError {
    #[error("mailbox dropped")]
    MailboxDropped,
    #[error("player dropped {0}")]
    PlayerDropped(PlayerId),
    #[error("no such player {0}")]
    NoPlayerFound(PlayerId),
    #[error("no players remaining")]
    AllPlayersDropped,
    #[error("game error: {0}")]
    Game(#[from] GameError)
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: HashMap<PlayerId, PlayerAddr>,
    owner: Option<PlayerId>,
    game: Option<Game>
}

pub struct RoomState {
    pub code: RoomCode,
    pub players: Vec<PlayerId>,
    pub owner: Option<PlayerId>,
    pub game: Option<GameSnapshot>
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
            game: None
        };
    }

    fn start_game(&mut self, requestor_id: PlayerId, difficulty: GameDifficulty) -> Result<(), GameError> {
        if Some(requestor_id) != self.owner {
            
        }
        let game = match Game::new(difficulty, random()) {
            Ok(game) => game,
            Err(e) => return Err(e),
        };
        self.game = Some(game);
        Ok(())
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
            owner: self.owner.to_owned(),
            game: match &self.game {
                Some(game) => Some(game.snapshot().clone()),
                None => None,
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
                    match self.handle_mailbox(msg).await {
                        Ok(()) => continue,
                        Err(mut remainder) => return Err(remainder.pop().unwrap().into()),
                    }
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

    async fn handle_mailbox(&mut self, msg: RoomMessage) -> Result<(), Vec<RoomError>> {
        let player_id = msg.id;
        let mut errs = Vec::new();
        match msg.command {
            PlayerCommand::Join { handle } => {
                if let None = self.game {
                    self.register_player(player_id, handle.clone());
                } else {
                    let _ = handle.send(SessionMessage::Kicked { reason: "game has already started".to_string() }).await;
                }
            },
            PlayerCommand::Leave => {
                let addr = match self.drop_player(&player_id).await {
                    Ok(addr) => addr,
                    Err(_) => return Ok(())
                };
                let _ = addr.send(SessionMessage::Kicked { reason: "player left".to_string() });
            },
            PlayerCommand::StartGame { difficulty } => {
                match self.start_game(player_id, difficulty) {
                    Ok(_) => (),
                    Err(e) => errs.push(RoomError::Game(e)),
                };
            },
        }
        
        match self.broadcast_state().await {
            Ok(()) => (),
            Err(mut broadcast_errs) => errs.append(&mut broadcast_errs),
        }

        if errs.len() > 0 {
            let mut to_drop = Vec::new();
            let remainder = errs.iter().filter_map(|err| {
                match err {
                    RoomError::PlayerDropped(id) => {
                        to_drop.push(id);
                        return None;
                    },
                    _ => return Some(err)
                }
            }).cloned().collect::<Vec<RoomError>>();
            if remainder.len() > 0 {
                return Err(remainder);
            } else {
                for id in to_drop {
                    let addr = match self.drop_player(id).await {
                        Ok(addr) => addr,
                        Err(_) => continue
                    };
                    let _ = addr.send(SessionMessage::Kicked { reason: "player dropped".to_string() }).await;
                }
                let _ = self.broadcast_state().await;
            };
        }

        Ok(())
    }

    async fn register_new_owner(&mut self) -> Result<PlayerId, RoomError> {
        let new_owner = match self.players.keys().next() {
            Some(id) => id.clone(),
            None => return Err(RoomError::AllPlayersDropped)
        };

        self.owner = Some(new_owner.to_owned());
        return Ok(new_owner);
    }

    async fn drop_player(&mut self, id: &PlayerId) -> Result<PlayerAddr, RoomError> {
        if Some(id.clone()) == self.owner {
            self.owner = None;
            self.register_new_owner().await?;
        }
        
        match self.players.remove(id) {
            Some(addr) => return Ok(addr),
            None => return Err(RoomError::NoPlayerFound(id.to_string())),
        };
    }

    async fn broadcast_state(&mut self) -> Result<(), Vec<RoomError>> {
        let state = match self.state() {
            Ok(state) => state,
            Err(err) => return Err(vec![err])
        };

        return match self.broadcast_message(state.into()).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        };
    }

    async fn broadcast_message(&mut self, msg: SessionMessage) -> Result<(), Vec<RoomError>> {
        let ids: Vec<PlayerId> = self.players.keys().map(ToString::to_string).collect();
        let mut errors = Vec::new();
        for id in ids {
            match self.send_player(&id, msg.clone()).await {
                Ok(()) => continue,
                Err(e) => {
                    errors.push(e);
                },
            }
        };
        return match errors.len() {
            0 => Ok(()),
            _ => Err(errors)
        }
    }

    async fn send_player(&mut self, id: &PlayerId, msg: SessionMessage) -> Result<(), RoomError> {
        let addr: &PlayerAddr = match self.players.get_mut(id) {
            Some(addr) => addr,
            None => return Err(RoomError::NoPlayerFound(id.clone()).into()),
        };
        
        return match addr.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(RoomError::PlayerDropped(id.clone()).into())
        }
    }
}