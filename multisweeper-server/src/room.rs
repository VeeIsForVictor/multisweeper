use std::collections::HashMap;

use anyhow::Result;
use multisweeper_core::{Game, GameDifficulty, GameError, GameSnapshot};
use rand::random;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    protocol::{
        room::{PlayerCommand, RoomMessage},
        session::SessionMessage,
    },
    session::{PlayerAddr, PlayerId},
};

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
    #[error("only the room owner can start the game")]
    NotOwner,
    #[error("game has already started")]
    GameAlreadyStarted,
    #[error("game has not started")]
    NoGame,
    #[error("no players remaining")]
    AllPlayersDropped,
    #[error("game error: {0}")]
    Game(#[from] GameError),
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: HashMap<PlayerId, PlayerAddr>,
    owner: Option<PlayerId>,
    game: Option<Game>,
}

pub struct RoomState {
    pub code: RoomCode,
    pub players: Vec<PlayerId>,
    pub owner: Option<PlayerId>,
    pub game: Option<GameSnapshot>,
}

enum RoomEvent {
    Session(Option<RoomMessage>),
}

impl Room {
    pub fn new(code: RoomCode) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        Room {
            code,
            mailbox: receiver,
            addr: sender,
            players: HashMap::new(),
            owner: None,
            game: None,
        }
    }

    fn start_game(
        &mut self,
        requestor_id: PlayerId,
        difficulty: GameDifficulty,
    ) -> Result<(), RoomError> {
        if Some(requestor_id) != self.owner {
            return Err(RoomError::NotOwner);
        }
        if self.game.is_some() {
            return Err(RoomError::GameAlreadyStarted);
        }
        self.game = Some(Game::new(difficulty, random())?);
        Ok(())
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomAddr {
        self.addr.clone()
    }

    pub fn state(&self) -> Result<RoomState, RoomError> {
        Ok(RoomState {
            code: self.code().to_string(),
            players: self.players.keys().map(ToString::to_string).collect(),
            owner: self.owner.to_owned(),
            game: self.game.as_ref().map(|game| game.snapshot().clone()),
        })
    }

    fn register_player(&mut self, id: PlayerId, addr: PlayerAddr) {
        if self.owner.is_none() {
            self.owner = Some(id.clone());
        }
        self.players.insert(id, addr);
    }

    pub async fn handle_connection(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
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

    async fn resolve_mailbox_errs(&mut self, mut errs: Vec<RoomError>) -> Vec<RoomError> {
        if !errs.is_empty() {
            let mut to_drop = Vec::new();
            let remainder = errs
                .iter()
                .filter(|err| {
                    if let RoomError::PlayerDropped(id) = err {
                        to_drop.push(id);
                        false
                    } else {
                        true
                    }
                })
                .cloned()
                .collect::<Vec<RoomError>>();
            for id in to_drop {
                let addr = match self.drop_player(id).await {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };
                let _ = addr
                    .send(SessionMessage::Kicked {
                        reason: "player dropped".to_string(),
                    })
                    .await;
            }
            let _ = self.broadcast_state().await;
            errs = remainder;
        }
        return errs;
    }

    async fn handle_mailbox(&mut self, msg: RoomMessage) -> Result<(), Vec<RoomError>> {
        let player_id = msg.id.clone();
        let mut errs = Vec::new();
        match msg.command {
            PlayerCommand::Join { handle } => {
                if self.game.is_none() {
                    self.register_player(player_id, handle.clone());
                } else {
                    let _ = handle
                        .send(SessionMessage::Kicked {
                            reason: "game has already started".to_string(),
                        })
                        .await;
                }
            }
            PlayerCommand::Leave => {
                let addr = match self.drop_player(&player_id).await {
                    Ok(addr) => addr,
                    Err(_) => return Ok(()),
                };
                let _ = addr
                    .send(SessionMessage::Kicked {
                        reason: "player left".to_string(),
                    })
                    .await;
            }
            PlayerCommand::StartGame { difficulty } => {
                match self.start_game(player_id.clone(), difficulty) {
                    Ok(_) => {
                        if let Err(mut send_errors) =
                            self.broadcast_message(SessionMessage::GameStarted).await
                        {
                            errs.append(&mut send_errors);
                        }
                    }
                    Err(e) => {
                        self.send_player_error(&player_id, e.to_string()).await;
                    }
                };
            }
            PlayerCommand::GameAction { action } => {
                let result = match self.game.as_mut() {
                    Some(game) => game.handle_action(action).cloned().map_err(RoomError::from),
                    None => Err(RoomError::NoGame),
                };

                match result {
                    Ok(_) => {
                        match self.state() {
                            Ok(state) => if let Err(error) = self.send_player(&player_id, state.into()).await {
                                errs.push(error);
                            },
                            Err(error) => {
                                errs.push(error);
                            }
                        }
                    },
                    Err(error) => self.send_player_error(&player_id, error.to_string()).await,
                }
            }
            PlayerCommand::GameQuery => match self.state() {
                Ok(state) => {
                    if let Err(error) = self.send_player(&player_id, state.into()).await {
                        errs.push(error);
                    }
                }
                Err(error) => errs.push(error),
            },
        }

        match self.broadcast_state().await {
            Ok(()) => (),
            Err(mut broadcast_errs) => errs.append(&mut broadcast_errs),
        }

        let remainder = self.resolve_mailbox_errs(errs).await;

        return match remainder.len() {
            0 => Err(remainder),
            _ => Ok(())
        }
    }

    async fn drop_player(&mut self, id: &PlayerId) -> Result<PlayerAddr, RoomError> {
        let addr = self
            .players
            .remove(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.to_string()))?;

        if Some(id.clone()) == self.owner {
            self.owner = self.players.keys().next().cloned();
        }

        Ok(addr)
    }

    async fn broadcast_state(&mut self) -> Result<(), Vec<RoomError>> {
        let state = match self.state() {
            Ok(state) => state,
            Err(err) => return Err(vec![err]),
        };

        match self.broadcast_message(state.into()).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn broadcast_message(&mut self, msg: SessionMessage) -> Result<(), Vec<RoomError>> {
        let ids: Vec<PlayerId> = self.players.keys().map(ToString::to_string).collect();
        let mut errors = Vec::new();
        for id in ids {
            match self.send_player(&id, msg.clone()).await {
                Ok(()) => continue,
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        match errors.len() {
            0 => Ok(()),
            _ => Err(errors),
        }
    }

    async fn send_player(&mut self, id: &PlayerId, msg: SessionMessage) -> Result<(), RoomError> {
        let addr: &PlayerAddr = match self.players.get_mut(id) {
            Some(addr) => addr,
            None => return Err(RoomError::NoPlayerFound(id.clone())),
        };

        return match addr.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(RoomError::PlayerDropped(id.clone())),
        };
    }

    async fn send_player_error(&mut self, id: &PlayerId, reason: String) {
        let _ = self.send_player(id, SessionMessage::Error { reason }).await;
    }
}
