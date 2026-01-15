use std::collections::HashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;
use tokio_stream::{StreamMap, wrappers::ReceiverStream};

use crate::ws::{PlayerId, player, protocol::{ClientMessage, PlayerConnection, ServerMessage}};

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
}

impl Lobby {
    pub fn new(host_id: PlayerId, host_connection: PlayerConnection, action_rcr: Receiver<ClientMessage>, code: LobbyCode) -> Self {
        let mut lobby = Lobby {
            code,
            players: HashMap::new(),
            player_streams: StreamMap::new(),
            host_id: host_id.clone(),
            status: LobbyStatus::Waiting
        };

        lobby.players.insert(host_id.clone(), host_connection);
        lobby.player_streams.insert(host_id, ReceiverStream::from(action_rcr));

        return lobby;
    }

    pub fn register_player(&mut self, player_id: PlayerId, player_connection: PlayerConnection, action_rcr: Receiver<ClientMessage>) -> PlayerId {
        self.players.insert(player_id.clone(), player_connection);
        self.player_streams.insert(player_id.clone(), ReceiverStream::from(action_rcr));
        return player_id;
    }

    pub fn deregister_player(&mut self, player_id: PlayerId) -> Option<(PlayerId, PlayerConnection, Receiver<ClientMessage>)> {
        let conn = match self.players.remove(&player_id) {
            Some(conn) => conn,
            None => return None
        };
        let action_rcr = match self.player_streams.remove(&player_id) {
            Some(rcr) => rcr,
            None => return None
        };
        return Some((player_id, conn, action_rcr.into_inner()));
    }

    pub fn start_game(&mut self) {
        self.status = LobbyStatus::Starting;
    }

    pub async fn broadcast_state(&mut self) {
        let players_list: Vec<PlayerId> = self.players.keys().cloned().collect();
        self.broadcast_message(ServerMessage::LobbyState {
            code: self.code.clone(),
            players: players_list.clone(), 
            host_id: self.host_id.clone(), 
            status: self.status.clone()
        }).await;
    }

    pub async fn broadcast_message(&mut self, msg: ServerMessage) {
        for player in self.players.values() {
            player.message_sdr.send(msg.clone()).await;
        }
    }

    pub async fn next_client_message(&mut self) -> Option<(PlayerId, ClientMessage)> {
        return self.player_streams.next().await;
    }

    pub fn get_host_id(&self) -> PlayerId {
        return self.host_id.clone();
    }

    pub fn get_code(&self) -> &LobbyCode {
        &self.code
    }
}