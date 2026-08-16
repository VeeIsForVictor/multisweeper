use std::collections::HashMap;

use anyhow::Result;
use multisweeper_core::{Game, GameActionResult, GameDifficulty, GameError, GameSnapshot};
use rand::random;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    protocol::{
        room::{PlayerCommand, RoomMessage},
        session::{MatchState, PlayerState, PlayerView, SessionMessage},
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
    #[error("game has ended")]
    GameEnded,
    #[error("player {0} is spectating")]
    PlayerIsSpectating(PlayerId),
    #[error("player {0} has been eliminated")]
    PlayerEliminated(PlayerId),
    #[error("no players remaining")]
    AllPlayersDropped,
    #[error("game error: {0}")]
    Game(#[from] GameError),
}

struct PlayerRecord {
    address: PlayerAddr,
    state: PlayerState,
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: HashMap<PlayerId, PlayerRecord>,
    owner: Option<PlayerId>,
    game: Option<Game>,
    match_state: MatchState,
}

pub struct RoomState {
    pub code: RoomCode,
    pub players: Vec<PlayerView>,
    pub owner: Option<PlayerId>,
    pub match_state: MatchState,
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
            match_state: MatchState::Waiting,
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
        if self.match_state != MatchState::Waiting {
            return Err(RoomError::GameAlreadyStarted);
        }
        self.game = Some(Game::new(difficulty, random())?);
        self.match_state = MatchState::Playing;
        for player in self.players.values_mut() {
            player.state = PlayerState::Playing;
        }
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
            players: self
                .players
                .iter()
                .map(|(id, player)| PlayerView {
                    id: id.clone(),
                    state: player.state.clone(),
                })
                .collect(),
            owner: self.owner.to_owned(),
            match_state: self.match_state.clone(),
            game: self.game.as_ref().map(|game| game.snapshot().clone()),
        })
    }

    fn register_player(&mut self, id: PlayerId, addr: PlayerAddr) {
        if self.owner.is_none() {
            self.owner = Some(id.clone());
        }
        self.players.insert(
            id,
            PlayerRecord {
                address: addr,
                state: PlayerState::Spectator,
            },
        );
    }

    fn ensure_can_play(&self, id: &PlayerId) -> Result<(), RoomError> {
        match self.match_state {
            MatchState::Waiting => return Err(RoomError::NoGame),
            MatchState::Won | MatchState::NoWinner => return Err(RoomError::GameEnded),
            MatchState::Playing => {}
        }

        let player = self
            .players
            .get(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.clone()))?;
        match player.state {
            PlayerState::Spectator => Err(RoomError::PlayerIsSpectating(id.clone())),
            PlayerState::Eliminated => Err(RoomError::PlayerEliminated(id.clone())),
            PlayerState::Playing => Ok(()),
        }
    }

    fn mark_player_eliminated(&mut self, id: &PlayerId) -> Result<(), RoomError> {
        let player = self
            .players
            .get_mut(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.clone()))?;
        player.state = PlayerState::Eliminated;
        Ok(())
    }

    fn has_active_players(&self) -> bool {
        self.players
            .values()
            .any(|player| player.state == PlayerState::Playing)
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
                        Err(mut remainder) => {
                            if let Some(error) = remainder.pop() {
                                return Err(error.into());
                            }
                        }
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
        errs
    }

    async fn handle_mailbox(&mut self, msg: RoomMessage) -> Result<(), Vec<RoomError>> {
        let player_id = msg.id.clone();
        let mut errs = Vec::new();
        match msg.command {
            PlayerCommand::Join { handle } => {
                if self.match_state == MatchState::Waiting {
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
                let result = self.ensure_can_play(&player_id).and_then(|()| {
                    self.game
                        .as_mut()
                        .ok_or(RoomError::NoGame)?
                        .handle_action(action)
                        .cloned()
                        .map_err(RoomError::from)
                });

                match result {
                    Ok(snapshot) => {
                        match snapshot.action_result {
                            GameActionResult::Eliminated => {
                                if let Err(error) = self.mark_player_eliminated(&player_id) {
                                    errs.push(error);
                                } else if !self.has_active_players() {
                                    if let Some(game) = self.game.as_mut() {
                                        game.lose_game();
                                    }
                                    self.match_state = MatchState::NoWinner;
                                }
                            }
                            GameActionResult::Won => {
                                self.match_state = MatchState::Won;
                            }
                            GameActionResult::Applied
                            | GameActionResult::Stalled
                            | GameActionResult::Started => {}
                        }
                        match self.state() {
                            Ok(state) => {
                                if let Err(error) = self.send_player(&player_id, state.into()).await
                                {
                                    errs.push(error);
                                }
                            }
                            Err(error) => {
                                errs.push(error);
                            }
                        }
                    }
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

        match remainder.len() {
            0 => Ok(()),
            _ => Err(remainder),
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

        Ok(addr.address)
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
        let addr = match self.players.get_mut(id) {
            Some(addr) => addr,
            None => return Err(RoomError::NoPlayerFound(id.clone())),
        };

        match addr.address.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(RoomError::PlayerDropped(id.clone())),
        }
    }

    async fn send_player_error(&mut self, id: &PlayerId, reason: String) {
        let _ = self.send_player(id, SessionMessage::Error { reason }).await;
    }
}
