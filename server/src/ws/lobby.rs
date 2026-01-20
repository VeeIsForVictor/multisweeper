use std::collections::HashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;
use tokio_stream::{StreamMap, wrappers::ReceiverStream};

use crate::ws::{PlayerId, protocol::{ClientMessage, PlayerConnection, ServerMessage}};

pub type LobbyCode = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LobbyStatus {
    Waiting,
    Starting
}

pub struct Lobby {
    code: LobbyCode,
    players: HashMap<PlayerId, PlayerConnection>,
    player_streams: StreamMap<PlayerId, ReceiverStream<ClientMessage>>,
    pub host_id: PlayerId,
    pub status: LobbyStatus,
    pub next_host_id: Option<PlayerId>,
}

impl Lobby {
    pub fn new(host_id: PlayerId, host_connection: PlayerConnection, action_rcr: Receiver<ClientMessage>, code: LobbyCode) -> Self {
        let mut lobby = Lobby {
            code,
            players: HashMap::new(),
            player_streams: StreamMap::new(),
            host_id: host_id.clone(),
            status: LobbyStatus::Waiting,
            next_host_id: None,
        };

        lobby.players.insert(host_id.clone(), host_connection);
        lobby.player_streams.insert(host_id, ReceiverStream::from(action_rcr));

        lobby
    }

    pub fn register_player(&mut self, player_id: PlayerId, player_connection: PlayerConnection, action_rcr: Receiver<ClientMessage>) -> PlayerId {
        self.players.insert(player_id.clone(), player_connection);
        self.player_streams.insert(player_id.clone(), ReceiverStream::from(action_rcr));
        player_id
    }

    pub fn deregister_player(&mut self, player_id: &PlayerId) -> Option<(PlayerId, PlayerConnection)> {
        let player_data = match self.players.remove(player_id) {
            Some(data) => data,
            None => return None
        };
        let _ = self.player_streams.remove(player_id);

        if player_id == &self.host_id {
            self.promote_new_host();
        }

        Some((player_id.clone(), player_data))
    }

    fn promote_new_host(&mut self) {
        self.next_host_id = self.players.keys().next().cloned();
        if let Some(new_host) = &self.next_host_id {
            self.host_id = new_host.clone();
        }
    }

    pub fn is_host_disconnected(&self) -> bool {
        !self.players.contains_key(&self.host_id)
    }

    pub fn get_players(&self) -> Vec<PlayerId> {
        self.players.keys().cloned().collect()
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn start_game(&mut self) {
        self.status = LobbyStatus::Starting;
    }

    pub async fn broadcast_state(&mut self) {
        let players_list = self.get_players();
        self.broadcast_message(ServerMessage::LobbyState {
            code: self.code.clone(),
            players: players_list,
            host_id: self.host_id.clone(),
            status: self.status.clone()
        }).await;
    }

    pub async fn broadcast_message(&mut self, msg: ServerMessage) {
        let mut disconnected = Vec::new();
        for (id, player) in &self.players {
            if player.message_sdr.send(msg.clone()).await.is_err() {
                disconnected.push(id.clone());
            }
        }
        for id in disconnected {
            self.handle_disconnect(&id);
        }
    }

    pub async fn next_client_message(&mut self) -> Option<(PlayerId, ClientMessage)> {
        self.player_streams.next().await
    }

    pub fn handle_disconnect(&mut self, player_id: &PlayerId) {
        self.players.remove(player_id);
        let _ = self.player_streams.remove(player_id);

        if player_id == &self.host_id {
            self.promote_new_host();
        }
    }

    pub fn get_host_id(&self) -> PlayerId {
        self.host_id.clone()
    }

    pub fn get_code(&self) -> &LobbyCode {
        &self.code
    }
}
