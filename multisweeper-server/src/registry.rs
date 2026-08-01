use std::collections::HashMap;

use parking_lot::Mutex;
use triomphe::Arc;

use crate::room::RoomMailbox;

pub type RegistryHandle = Arc<Mutex<Registry>>;

pub struct Registry {
    entity_counter: u64,
    rooms: HashMap<String, RoomMailbox>
}

impl Registry {
    pub fn new() -> Self {
        return Registry {
            entity_counter: 0,
            rooms: HashMap::new()
        }
    }

    fn generate_name(&mut self, prefix: &str) -> String {
        let id = self.entity_counter;
        self.entity_counter += 1;
        String::from(format!("{prefix}{id:0>5}"))
    }

    pub fn register_player(&mut self) -> String {
        self.generate_name("P")
    }

    pub fn register_lobby(&mut self, handle: RoomMailbox) -> String {
        let name = self.generate_name("L");
        self.rooms.insert(name.clone(), handle);
        return name;
    }
}