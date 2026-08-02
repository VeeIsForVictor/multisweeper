use tokio::sync::oneshot::Sender;

use crate::{registry::RegistryError, room::{RoomAddr, RoomCode}, session::PlayerId};

pub enum RegistryMessage {
    CreateLobby(Sender<RoomAddr>),
    RequestLobby{code: RoomCode, reply: Sender<Result<RoomAddr, RegistryError>>},
    QueryLobbies(Sender<Vec<RoomCode>>),
    CreatePlayer(Sender<PlayerId>)
}