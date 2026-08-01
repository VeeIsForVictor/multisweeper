use std::collections::HashMap;

use anyhow::Result;
use parking_lot::Mutex;
use thiserror::Error;
use triomphe::Arc;

use crate::room::{Room, RoomCode, RoomHandle, RoomMailbox};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("requested room with code {0} not found")]
    RoomNotFound(String)
}

pub type RegistryHandle = Arc<Mutex<Registry>>;

pub struct Registry {
    entity_counter: u64,
    rooms: HashMap<String, RoomHandle>,
}

impl Registry {
    pub fn new() -> Self {
        return Registry {
            entity_counter: 0,
            rooms: HashMap::new(),
        };
    }

    fn generate_name(&mut self, prefix: &str) -> String {
        let id = self.entity_counter;
        self.entity_counter += 1;
        String::from(format!("{prefix}{id:0>5}"))
    }

    pub fn register_player(&mut self) -> String {
        self.generate_name("P")
    }

    pub fn register_lobby(&mut self) -> (String, RoomHandle) {
        let code = self.generate_name("L");
        let room = Room::new(code.clone());
        self.rooms.insert(code.clone(), room.request_handle());
        return (code, room.request_handle());
    }

    pub fn request_lobby(&mut self, code: RoomCode) -> Result<RoomHandle, RegistryError> {
        match self.rooms.get(&code) {
            Some(handle) => Ok(handle.clone()),
            None => Err(RegistryError::RoomNotFound(code).into())
        }
    }
}
