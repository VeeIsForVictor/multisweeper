use std::collections::HashMap;

use anyhow::Result;
use multisweeper_core::{Game, GameActionResult, GameDifficulty, GameError, GameSnapshot};
use rand::random;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, error, info, warn};

use crate::{
    protocol::{
        room::{PlayerCommand, RoomMessage},
        session::{
            MatchState as ProtocolMatchState, MatchView, PlayerState, PlayerView, SessionMessage,
        },
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
    #[error("player {0} is not current player")]
    PlayerNotCurrent(PlayerId),
    #[error("no players remaining")]
    AllPlayersDropped,
    #[error("game error: {0}")]
    Game(#[from] GameError),
}

struct PlayerRecord {
    address: PlayerAddr,
    state: PlayerState,
}

struct PlayingMatch {
    game: Game,
    participants: Vec<PlayerId>,
    last_player: Option<PlayerId>,
    current_player: PlayerId,
}

enum RoomMatchState {
    Waiting,
    Playing(PlayingMatch),
    Won { final_snapshot: GameSnapshot },
    NoWinner { final_snapshot: GameSnapshot },
}

pub struct Room {
    code: RoomCode,
    mailbox: RoomMailbox,
    addr: RoomAddr,
    players: HashMap<PlayerId, PlayerRecord>,
    owner: Option<PlayerId>,
    match_state: RoomMatchState,
}

pub struct RoomState {
    pub code: RoomCode,
    pub players: Vec<PlayerView>,
    pub owner: Option<PlayerId>,
    pub match_state: MatchView,
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
            match_state: RoomMatchState::Waiting,
        }
    }

    fn start_game(
        &mut self,
        requestor_id: PlayerId,
        difficulty: GameDifficulty,
    ) -> Result<(), RoomError> {
        if Some(requestor_id.clone()) != self.owner {
            return Err(RoomError::NotOwner);
        }
        if !matches!(self.match_state, RoomMatchState::Waiting) {
            return Err(RoomError::GameAlreadyStarted);
        }
        let game = Game::new(difficulty, random())?;
        let participants = self.players.keys().cloned().collect();
        for player in self.players.values_mut() {
            player.state = PlayerState::Playing;
        }
        self.match_state = RoomMatchState::Playing(PlayingMatch {
            game,
            participants,
            last_player: None,
            current_player: requestor_id.clone(),
        });
        info!(
            target: "multisweeper.room.match_started",
            room_code = %self.code,
            player_id = %requestor_id,
            "match started"
        );
        Ok(())
    }

    pub fn code(&self) -> &RoomCode {
        &self.code
    }

    pub fn request_handle(&self) -> RoomAddr {
        self.addr.clone()
    }

    fn get_player_queue(&self) -> Vec<PlayerView> {
        self.players
            .iter()
            .map(|(id, player)| PlayerView {
                id: id.clone(),
                state: player.state.clone(),
            })
            .collect()
    }

    fn match_view(&self) -> MatchView {
        match &self.match_state {
            RoomMatchState::Waiting => MatchView {
                state: ProtocolMatchState::Waiting,
                game: None,
            },
            RoomMatchState::Playing(active_match) => MatchView {
                state: ProtocolMatchState::Playing {
                    last_player: active_match.last_player.clone(),
                    current_player: active_match.current_player.clone(),
                },
                game: Some(active_match.game.snapshot().clone()),
            },
            RoomMatchState::Won { final_snapshot } => MatchView {
                state: ProtocolMatchState::Won,
                game: Some(final_snapshot.clone()),
            },
            RoomMatchState::NoWinner { final_snapshot } => MatchView {
                state: ProtocolMatchState::NoWinner,
                game: Some(final_snapshot.clone()),
            },
        }
    }

    pub fn state(&self) -> Result<RoomState, RoomError> {
        Ok(RoomState {
            code: self.code().to_string(),
            players: self.get_player_queue(),
            owner: self.owner.to_owned(),
            match_state: self.match_view(),
        })
    }

    fn register_player(&mut self, id: PlayerId, addr: PlayerAddr) {
        if self.owner.is_none() {
            self.owner = Some(id.clone());
        }
        self.players.insert(
            id.clone(),
            PlayerRecord {
                address: addr,
                state: PlayerState::Spectator,
            },
        );
        info!(
            target: "multisweeper.room.player_joined",
            room_code = %self.code,
            player_id = %id,
            player_state = "spectator",
            "player joined room"
        );
    }

    fn ensure_can_play(&self, id: &PlayerId) -> Result<(), RoomError> {
        let player = self
            .players
            .get(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.clone()))?;
        match player.state {
            PlayerState::Spectator => return Err(RoomError::PlayerIsSpectating(id.clone())),
            PlayerState::Eliminated => return Err(RoomError::PlayerEliminated(id.clone())),
            PlayerState::Playing => {}
        }

        match &self.match_state {
            RoomMatchState::Waiting => return Err(RoomError::NoGame),
            RoomMatchState::Won { .. } | RoomMatchState::NoWinner { .. } => {
                return Err(RoomError::GameEnded);
            }
            RoomMatchState::Playing(active_match) => {
                if &active_match.current_player != id {
                    return Err(RoomError::PlayerNotCurrent(id.clone()));
                }
            }
        }

        Ok(())
    }

    fn move_to_next_player(&mut self) -> Result<(), RoomError> {
        let (current_player, participants) = match &self.match_state {
            RoomMatchState::Playing(active_match) => (
                active_match.current_player.clone(),
                active_match.participants.clone(),
            ),
            RoomMatchState::Waiting => return Err(RoomError::NoGame),
            RoomMatchState::Won { .. } | RoomMatchState::NoWinner { .. } => {
                return Err(RoomError::GameEnded);
            }
        };

        let current_index = participants
            .iter()
            .position(|player_id| player_id == &current_player)
            .ok_or_else(|| RoomError::NoPlayerFound(current_player.clone()))?;
        let active_players = participants
            .iter()
            .filter(|player_id| {
                self.players
                    .get(*player_id)
                    .is_some_and(|player| player.state == PlayerState::Playing)
            })
            .cloned()
            .collect::<Vec<_>>();
        let next_player = (1..=participants.len())
            .map(|offset| &participants[(current_index + offset) % participants.len()])
            .find(|player_id| active_players.contains(player_id))
            .cloned()
            .ok_or(RoomError::AllPlayersDropped)?;

        if let RoomMatchState::Playing(active_match) = &mut self.match_state {
            active_match.last_player = Some(current_player);
            active_match.current_player = next_player;
            info!(
                target: "multisweeper.room.turn_changed",
                room_code = %self.code,
                player_id = %active_match.current_player,
                previous_player = ?active_match.last_player,
                "turn changed"
            );
        }
        Ok(())
    }

    fn mark_player_eliminated(&mut self, id: &PlayerId) -> Result<(), RoomError> {
        let player = self
            .players
            .get_mut(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.clone()))?;
        player.state = PlayerState::Eliminated;
        info!(
            target: "multisweeper.room.player_eliminated",
            room_code = %self.code,
            player_id = %id,
            "player eliminated"
        );
        Ok(())
    }

    fn has_active_players(&self) -> bool {
        self.players
            .values()
            .any(|player| player.state == PlayerState::Playing)
    }

    fn finish_without_winner(&mut self) -> Result<(), RoomError> {
        let match_state = std::mem::replace(&mut self.match_state, RoomMatchState::Waiting);
        let RoomMatchState::Playing(mut active_match) = match_state else {
            self.match_state = match_state;
            return Err(RoomError::GameEnded);
        };
        active_match.game.lose_game();
        self.match_state = RoomMatchState::NoWinner {
            final_snapshot: active_match.game.snapshot().clone(),
        };
        info!(
            target: "multisweeper.room.match_finished",
            room_code = %self.code,
            outcome = "no_winner",
            "match finished"
        );
        Ok(())
    }

    #[tracing::instrument(name = "room.lifecycle", skip_all, fields(room_code = %self.code))]
    pub async fn handle_connection(mut self) -> Result<()> {
        let result = self.event_loop().await;
        if let Err(error) = &result {
            error!(
                target: "multisweeper.room.failed",
                room_code = %self.code,
                error = %error,
                "room task terminated with an error"
            );
        }
        result
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
                        Ok(()) => (),
                        Err(mut remainder) => {
                            if let Some(error) = remainder.pop() {
                                return Err(error.into());
                            }
                        }
                    }
                }
            }

            if self.players.len() == 0 {
                info!(
                    target: "multisweeper.room.room_closed",
                    room_code = %self.code,
                    "room task terminating gracefully due to no players"
                );
                return Ok(());
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
        debug!(
            target: "multisweeper.room.command_received",
            room_code = %self.code,
            player_id = %player_id,
            command = player_command_name(&msg.command),
            "room command received"
        );
        let mut errs = Vec::new();
        match msg.command {
            PlayerCommand::Join { handle } => {
                if matches!(self.match_state, RoomMatchState::Waiting) {
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
                    let RoomMatchState::Playing(active_match) = &mut self.match_state else {
                        return Err(match self.match_state {
                            RoomMatchState::Waiting => RoomError::NoGame,
                            RoomMatchState::Won { .. } | RoomMatchState::NoWinner { .. } => {
                                RoomError::GameEnded
                            }
                            RoomMatchState::Playing(_) => unreachable!(),
                        });
                    };
                    active_match
                        .game
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
                                    if let Err(error) = self.finish_without_winner() {
                                        errs.push(error);
                                    }
                                } else if let Err(error) = self.move_to_next_player() {
                                    errs.push(error);
                                }
                            }
                            GameActionResult::Won => {
                                let match_state = std::mem::replace(
                                    &mut self.match_state,
                                    RoomMatchState::Waiting,
                                );
                                if let RoomMatchState::Playing(active_match) = match_state {
                                    self.match_state = RoomMatchState::Won {
                                        final_snapshot: active_match.game.snapshot().clone(),
                                    };
                                    info!(
                                        target: "multisweeper.room.match_finished",
                                        room_code = %self.code,
                                        outcome = "won",
                                        "match finished"
                                    );
                                } else {
                                    self.match_state = match_state;
                                    errs.push(RoomError::GameEnded);
                                }
                            }
                            GameActionResult::Applied
                            | GameActionResult::Stalled
                            | GameActionResult::Started => {
                                if let Err(error) = self.move_to_next_player() {
                                    errs.push(error);
                                }
                            }
                        }
                        match self.state() {
                            Ok(_) => (),
                            Err(error) => {
                                errs.push(error);
                            }
                        }
                    }
                    Err(error) => {
                        debug!(
                            target: "multisweeper.room.action_rejected",
                            room_code = %self.code,
                            player_id = %player_id,
                            error_type = room_error_name(&error),
                            "game action rejected"
                        );
                        self.send_player_error(&player_id, error.to_string()).await;
                    }
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
        let record = self
            .players
            .remove(id)
            .ok_or_else(|| RoomError::NoPlayerFound(id.to_string()))?;

        if Some(id.clone()) == self.owner {
            self.owner = self.players.keys().next().cloned();
        }

        info!(
            target: "multisweeper.room.player_left",
            room_code = %self.code,
            player_id = %id,
            "player left room"
        );

        if let RoomMatchState::Playing(active_match) = &self.match_state
            && &active_match.current_player == id
        {
            if self.has_active_players() {
                let _ = self.move_to_next_player();
            } else {
                let _ = self.finish_without_winner();
            }
        }

        Ok(record.address)
    }

    #[tracing::instrument(
        name = "room.broadcast_state",
        skip_all,
        fields(room_code = %self.code)
    )]
    async fn broadcast_state(&mut self) -> Result<(), Vec<RoomError>> {
        debug!(
            target: "multisweeper.room.state_broadcast",
            room_code = %self.code,
            player_count = self.players.len(),
            "room state broadcast"
        );
        let state = match self.state() {
            Ok(state) => state,
            Err(err) => return Err(vec![err]),
        };

        match self.broadcast_message(state.into()).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[tracing::instrument(
        name = "room.broadcast_message",
        skip_all,
        fields(room_code = %self.code)
    )]
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

    #[tracing::instrument(
        name = "room.send_player",
        skip_all,
        fields(room_code = %self.code, player_id = %id)
    )]
    async fn send_player(&mut self, id: &PlayerId, msg: SessionMessage) -> Result<(), RoomError> {
        let addr = match self.players.get_mut(id) {
            Some(addr) => addr,
            None => return Err(RoomError::NoPlayerFound(id.clone())),
        };

        match addr.address.send(msg).await {
            Ok(()) => Ok(()),
            Err(_error) => {
                warn!(
                    target: "multisweeper.room.player_delivery_failed",
                    room_code = %self.code,
                    player_id = %id,
                    error_type = "player_dropped",
                    "player mailbox delivery failed"
                );
                Err(RoomError::PlayerDropped(id.clone()))
            }
        }
    }

    async fn send_player_error(&mut self, id: &PlayerId, reason: String) {
        let _ = self.send_player(id, SessionMessage::Error { reason }).await;
    }
}

fn player_command_name(command: &PlayerCommand) -> &'static str {
    match command {
        PlayerCommand::Join { .. } => "join",
        PlayerCommand::Leave => "leave",
        PlayerCommand::StartGame { .. } => "start_game",
        PlayerCommand::GameAction { .. } => "game_action",
        PlayerCommand::GameQuery => "game_query",
    }
}

fn room_error_name(error: &RoomError) -> &'static str {
    match error {
        RoomError::MailboxDropped => "mailbox_dropped",
        RoomError::PlayerDropped(_) => "player_dropped",
        RoomError::NoPlayerFound(_) => "no_player_found",
        RoomError::NotOwner => "not_owner",
        RoomError::GameAlreadyStarted => "game_already_started",
        RoomError::NoGame => "no_game",
        RoomError::GameEnded => "game_ended",
        RoomError::PlayerIsSpectating(_) => "player_is_spectating",
        RoomError::PlayerEliminated(_) => "player_eliminated",
        RoomError::PlayerNotCurrent(_) => "player_not_current",
        RoomError::AllPlayersDropped => "all_players_dropped",
        RoomError::Game(_) => "game_error",
    }
}
