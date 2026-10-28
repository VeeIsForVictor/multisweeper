use tokio::sync::oneshot::Sender;

use crate::{
    registry::RegistryError,
    room::{RoomAddr, RoomCode},
    session::PlayerId,
};

#[derive(Clone)]
pub struct PlayerRegistration {
    pub id: PlayerId,
    pub resume_token: String,
    pub generation: u64,
}

pub struct ResumedPlayer {
    pub id: PlayerId,
    pub room: Option<RoomAddr>,
    pub generation: u64,
}

pub enum RegistryMessage {
    CreateLobby(Sender<RoomAddr>),
    RequestLobby {
        code: RoomCode,
        reply: Sender<Result<RoomAddr, RegistryError>>,
    },
    QueryLobbies(Sender<Vec<RoomCode>>),
    CreatePlayer(Sender<PlayerRegistration>),
    RegisterPlayerRoom {
        player_id: PlayerId,
        resume_token: String,
        room: RoomAddr,
    },
    ClearPlayerRoom {
        resume_token: String,
    },
    DetachPlayer {
        resume_token: String,
        generation: u64,
    },
    ResumePlayer {
        resume_token: String,
        reply: Sender<Result<ResumedPlayer, RegistryError>>,
    },
}
