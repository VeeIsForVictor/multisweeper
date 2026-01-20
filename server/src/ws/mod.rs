use std::collections::{HashMap, VecDeque};
use std::result;
use std::sync::Arc;
use std::thread::current;
use std::time::Duration;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};

use tokio::time::{Instant, sleep, sleep_until};
use tracing::{info, warn};
use crate::game::Game;
use crate::ws::lobby::{Lobby, LobbyCode, LobbyStatus};
use crate::ws::lobby_game::LobbyGame;
use crate::ws::protocol::{ClientMessage, LobbyAction, LobbyCommand, PlayerConnection, PlayerResult, ServerMessage};

mod lobby;
mod lobby_game;
pub mod protocol;
pub mod player;

pub type PlayerId = String;

#[derive(Debug)]
pub struct LobbyHandle {
    pub cmd_sdr: Sender<LobbyCommand>
}

#[derive(Debug)]
pub struct SharedState {
    lobbies: HashMap<LobbyCode, LobbyHandle>,
    idle_players: HashMap<PlayerId, PlayerConnection>,
    latest_player_id_number: u32,
    rng: Pcg64
}

impl SharedState {
    pub fn new(seed: u64) -> Self {
        SharedState {
            lobbies: HashMap::new(),
            idle_players: HashMap::new(),
            latest_player_id_number: 0,
            rng: Pcg64::seed_from_u64(seed)
        }
    }

    pub fn new_player_name(&mut self) -> PlayerId {
        let player_id = format!("player {}", self.latest_player_id_number);
        self.latest_player_id_number += 1;
        return player_id;
    }

    pub fn register_player(&mut self, player_id: PlayerId, action_sdr: Sender<ClientMessage>, message_sdr: Sender<ServerMessage>) -> PlayerId {
        let connection = PlayerConnection { action_sdr: action_sdr, message_sdr: message_sdr };
        self.idle_players.insert(player_id.clone(), connection);
        return player_id;
    }

    pub fn register_lobby(&mut self, cmd_sdr: Sender<LobbyCommand>) -> LobbyCode {
        let lobby_code = self.rng.random_range(1000..=9999).to_string();
        self.lobbies.insert(lobby_code.clone(), LobbyHandle {
            cmd_sdr
        });
        return lobby_code;
    }

    pub fn get_lobby(&self, lobby_code: LobbyCode) -> Option<(LobbyCode, &Sender<LobbyCommand>)> {
        match self.lobbies.get(&lobby_code) {
            Some(handle) => Some((lobby_code, &handle.cmd_sdr)),
            None => None
        }
    }

    pub fn deregister_lobby(&mut self, lobby_code: &LobbyCode) -> Option<LobbyHandle> {
        self.lobbies.remove(lobby_code)
    }

    pub fn de_idle_player_by_id(&mut self, player_id: PlayerId) -> Option<(PlayerId, PlayerConnection)> {
        match self.idle_players.remove(&player_id) {
            Some(conn) => Some((player_id, conn)),
            None => None
        }
    }

    pub fn register_idle_player(&mut self, player_id: PlayerId, connection: PlayerConnection) {
        self.idle_players.insert(player_id, connection);
    }
}

pub async fn lobby_manager_task(
    mut cmd_rcr: Receiver<LobbyCommand>,
    host_player: (PlayerId, PlayerConnection),
    action_rcr: Receiver<ClientMessage>,
    code: String,
    state: Arc<Mutex<SharedState>>,
) {
    let (host_id, connection) = host_player;

    let mut lobby = Lobby::new(host_id, connection, action_rcr, code.clone());
    lobby.broadcast_state().await;

    loop {
        tokio::select! {
            cmd = cmd_rcr.recv() => {
                let Some(cmd) = cmd else {
                    info!("Lobby {} command channel closed, shutting down", lobby.get_code());
                    break;
                };

                match cmd {
                    LobbyCommand::AddPlayer { id, player_connection, action_rcr } => {
                        lobby.register_player(id.clone(), player_connection, action_rcr);
                    },
                    LobbyCommand::RemovePlayer { id, return_to_idle } => {
                        if let Some((player_id, connection)) = lobby.deregister_player(&id) {
                            if return_to_idle {
                                state.lock().await.register_idle_player(player_id, connection);
                            }
                        }
                    }
                }
            },

            act = lobby.next_client_message() => {
                match act {
                    Some((id, act)) => {
                        match act {
                            ClientMessage::LobbyClient(LobbyAction::StartGame) => {
                                if id == lobby.host_id {
                                    lobby.start_game();
                                }
                            },
                            _ => {
                                warn!("Unexpected message type received in lobby: {:?}", act);
                            }
                        }
                    },
                    None => {
                        info!("Lobby {} all player streams closed, shutting down", lobby.get_code());
                        break;
                    }
                }
            }
        }

        lobby.broadcast_state().await;
        if let LobbyStatus::Starting = lobby.status {
            lobby.broadcast_message(ServerMessage::GameStarted).await;
            lobby = game_manager_task(lobby).await;
        }
    }
    

    info!("Lobby {} shutting down, notifying players and cleaning up", code);
    let _ = lobby.broadcast_message(ServerMessage::Error {
        code: crate::ws::protocol::ErrorCode::LobbyNotFound,
        message: "Lobby has been closed".to_string()
    }).await;

    state.lock().await.deregister_lobby(&code);
}

pub async fn game_manager_task(mut lobby: Lobby) -> Lobby {
    let mut game = Game::new(
        crate::game::GameDifficulty::TEST,
        1234
    );

    let mut player_order = VecDeque::from(lobby.get_players());
    let game_info = game.info();
    
    lobby.broadcast_message(ServerMessage::GameInfo { 
        code: lobby.get_code().clone(), 
        width: game_info.width,
        height: game_info.height,
        number_of_mines: game_info.number_of_mines,
        seed: game_info.seed
    }).await;

    while let Some(current_player) = player_order.pop_front() {
        lobby.broadcast_message(ServerMessage::PlayerTurn(current_player.clone())).await;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut result = PlayerResult::PLAYING;

        loop {
            let timer = sleep_until(deadline);
    
            result = tokio::select! {
                action = lobby.next_player_message(current_player.clone()) => {
                    match action {
                        Some(action) => {
                            match game.handle_action(action.clone().into()) {
                                Ok(result) => {
                                    lobby.broadcast_message(ServerMessage::PlayerAction(current_player.clone(), action)).await;
                                    result.into()
                                },
                                Err(_) => {
                                    lobby.send_player_error(
                                        current_player.clone(), 
                                        crate::error::GameError::GameLogicError
                                    ).await;
                                    PlayerResult::STALLED
                                },
                            } 
                        },
                        None => PlayerResult::LOST,
                    }
                },
                _timeout = timer => {
                    PlayerResult::TIMEOUT
                },
            };

            if let PlayerResult::STALLED = result {
                warn!("Player {} stalled!", current_player.clone());
            } else {
                lobby.broadcast_message(ServerMessage::PlayerResult(current_player.clone(), result.clone())).await;
                break;
            }
        }

        if let PlayerResult::PLAYING = result {
            player_order.push_back(current_player.clone());
        }
    }
    
    return lobby;
}