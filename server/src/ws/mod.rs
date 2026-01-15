use std::collections::HashMap;
use std::sync::Arc;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};

use tracing::{info, warn};
use crate::ws::lobby::{Lobby, LobbyCode, LobbyStatus};
use crate::ws::protocol::{ClientMessage, LobbyAction, LobbyCommand, PlayerConnection, ServerMessage};

mod lobby;
pub mod protocol;
pub mod player;

pub type PlayerId = String;

#[derive(Debug)]
pub struct LobbyHandle {
    pub cmd_sdr: Sender<LobbyCommand>,
    pub player_count: usize,
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

    pub fn register_player(&mut self, action_sdr: Sender<ClientMessage>, message_sdr: Sender<ServerMessage>) -> PlayerId {
        let connection = PlayerConnection { action_sdr: action_sdr, message_sdr: message_sdr };
        let player_id: PlayerId = format!("player {}", self.latest_player_id_number);
        self.latest_player_id_number += 1;
        self.idle_players.insert(player_id.clone(), connection);
        return player_id;
    }

    pub fn register_lobby(&mut self, cmd_sdr: Sender<LobbyCommand>) -> LobbyCode {
        let lobby_code = self.rng.random_range(1000..=9999).to_string();
        self.lobbies.insert(lobby_code.clone(), LobbyHandle {
            cmd_sdr,
            player_count: 1,
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

    pub fn increment_lobby_player_count(&mut self, lobby_code: &LobbyCode) {
        if let Some(handle) = self.lobbies.get_mut(lobby_code) {
            handle.player_count += 1;
        }
    }

    pub fn decrement_lobby_player_count(&mut self, lobby_code: &LobbyCode) {
        if let Some(handle) = self.lobbies.get_mut(lobby_code) {
            handle.player_count = handle.player_count.saturating_sub(1);
        }
    }

    pub fn de_idle_player_by_id(&mut self, player_id: PlayerId) -> Option<(PlayerId, PlayerConnection)> {
        match self.idle_players.remove(&player_id) {
            Some(conn) => Some((player_id, conn)),
            None => None
        }
    }
}

#[tracing::instrument]
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
                        state.lock().await.increment_lobby_player_count(&code);
                    },
                    LobbyCommand::RemovePlayer(id) => {
                        lobby.deregister_player(&id);
                        state.lock().await.decrement_lobby_player_count(&code);
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
        }
    }

    info!("Lobby {} shutting down, notifying players and cleaning up", code);
    let _ = lobby.broadcast_message(ServerMessage::Error {
        code: crate::ws::protocol::ErrorCode::LobbyNotFound,
        message: "Lobby has been closed".to_string()
    }).await;

    state.lock().await.deregister_lobby(&code);
}