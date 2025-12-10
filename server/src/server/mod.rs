mod lobby;
pub mod protocol;

use protocol::LobbyStatus;

pub struct SharedState {

}

struct Lobby {
    players: Vec<String>,
    host_id: String,
    status: LobbyStatus
}