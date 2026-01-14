use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::ws::{lobby::{Lobby, LobbyCode, LobbyStatus}, protocol::{ClientMessage, LobbyAction, LobbyCommand, PlayerConnection, ServerMessage}};

mod lobby;
pub mod protocol;
pub mod player;

pub type PlayerId = String;

#[derive(Debug)]
pub struct SharedState {
    lobbies: HashMap<LobbyCode, mpsc::Sender<LobbyCommand>>,
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
        self.lobbies.insert(lobby_code.clone(), cmd_sdr);
        return lobby_code;
    }

    pub fn get_lobby(&self, lobby_code: LobbyCode) -> Option<(LobbyCode, &Sender<LobbyCommand>)> {
        match self.lobbies.get(&lobby_code) {
            Some(handle) => Some((lobby_code, handle)),
            None => None
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
pub async fn lobby_manager_task(mut cmd_rcr: Receiver<LobbyCommand>, host_player: (PlayerId, PlayerConnection), action_rcr: Receiver<ClientMessage>, code: String) {
    let (host_id, connection) = host_player;
    
    let mut lobby = Lobby::new(host_id, connection, action_rcr, code);
    lobby.broadcast_state().await;

    loop {
        tokio::select! {
            cmd = cmd_rcr.recv() => {
                let Some(cmd) = cmd else {
                    panic!("lobby's command sender dropped!")
                };

                match cmd {
                    LobbyCommand::AddPlayer { id, player_connection, action_rcr } => {
                        lobby.register_player(id, player_connection, action_rcr);
                    },
                    LobbyCommand::RemovePlayer(id) => {
                        lobby.deregister_player(id);
                        // TODO: handle returning player to idle
                    }
                }
            },

            act = lobby.next_client_message() => {
                let Some((id, act)) = act else {
                    panic!("lobby's player streams dropped!")
                };

                match act {
                    ClientMessage::LobbyClient(LobbyAction::StartGame) => {
                        if (id == lobby.host_id) {
                            lobby.start_game();
                        }
                    },
                    _ => {
                        panic!("Invalid client message passed")
                    }
                }
            }
        }
        
        lobby.broadcast_state().await;
        
        if let LobbyStatus::Starting = lobby.status {
            lobby.broadcast_message(ServerMessage::GameStarted).await;
        }
    }
}