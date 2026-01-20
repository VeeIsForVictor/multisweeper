use std::collections::HashMap;

use tokio_stream::{StreamMap, wrappers::ReceiverStream};

use crate::{game::Game, ws::{PlayerId, lobby::{Lobby, LobbyCode}, protocol::{ClientMessage, PlayerConnection}}};

pub struct GameStatus {
    
}

pub struct LobbyGame {
    code: LobbyCode,
    players: HashMap<PlayerId, PlayerConnection>,
    player_streams: StreamMap<PlayerId, ReceiverStream<ClientMessage>>,
    pub host_id: PlayerId,
    pub next_host_id: Option<PlayerId>,
}

impl From<Lobby> for LobbyGame {
    fn from(lobby: Lobby) -> Self {
        while lobby.player_count() > 0 {
            lobby.deregister_player(player_id);
        }
    }
}