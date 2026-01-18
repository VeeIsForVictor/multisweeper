use tokio::sync::mpsc::Receiver;

use crate::ws::{lobby::LobbyCode, protocol::ClientMessage};

#[derive(Debug)]
pub struct IdleState {
    pub action_rcr: Receiver<ClientMessage>,
}

#[derive(Debug)]
pub struct LobbyState {
    pub code: LobbyCode,
}

#[derive(Debug)]
pub enum ConnectionState {
    Idle(IdleState),
    Lobby(LobbyState),
    Game,
    Disconnected,
}

impl ConnectionState {
    pub fn is_idle(&self) -> bool {
        matches!(self, ConnectionState::Idle(_))
    }

    pub fn is_in_lobby(&self) -> Option<&LobbyCode> {
        match self {
            ConnectionState::Lobby(state) => Some(&state.code),
            _ => None,
        }
    }

    pub fn is_game(&self) -> bool {
        matches!(self, ConnectionState::Game)
    }

    pub fn is_disconnected(&self) -> bool {
        matches!(self, ConnectionState::Disconnected)
    }

    pub fn into_idle(self) -> Option<IdleState> {
        match self {
            ConnectionState::Idle(state) => Some(state),
            _ => None,
        }
    }

    pub fn into_lobby(self) -> Option<LobbyState> {
        match self {
            ConnectionState::Lobby(state) => Some(state),
            _ => None,
        }
    }

    pub fn take_action_rcr(&mut self) -> Option<&mut Receiver<ClientMessage>> {
        match self {
            ConnectionState::Idle(state) => Some(&mut state.action_rcr),
            _ => None,
        }
    }
}
