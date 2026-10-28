use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Result;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{Id as TaskId, JoinError, JoinSet};
use tracing::{debug, error, info, warn};

use crate::{
    protocol::registry::RegistryMessage,
    protocol::registry::{PlayerRegistration, ResumedPlayer},
    room::{Room, RoomAddr, RoomCode},
};

const RESUME_GRACE: Duration = Duration::from_secs(30);

struct PlayerLease {
    id: String,
    room: Option<RoomAddr>,
    expires_at: Option<Instant>,
    generation: u64,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("requested room with code {0} not found")]
    RoomNotFound(String),
    #[error("all senders dropped")]
    AddrDropped,
    #[error("reply failed")]
    ReplyFailed,
    #[error("player resume token was not found or has expired")]
    PlayerNotFound,
}

pub type RegistryMailbox = mpsc::Receiver<RegistryMessage>;
pub type RegistryAddr = mpsc::Sender<RegistryMessage>;

type ReplyHandle<T> = oneshot::Sender<T>;

pub enum RegistryEvent {
    Mailbox(Option<RegistryMessage>),
    RoomTask(Option<std::result::Result<(TaskId, Result<()>), JoinError>>),
}

pub struct Registry {
    entity_counter: u64,
    rooms: HashMap<String, RoomAddr>,
    room_codes_by_task: HashMap<TaskId, RoomCode>,
    room_tasks: JoinSet<Result<()>>,
    players: HashMap<String, PlayerLease>,
    mailbox: RegistryMailbox,
    addr: RegistryAddr,
}

impl Registry {
    pub fn new() -> Self {
        let (addr, mailbox) = mpsc::channel::<RegistryMessage>(10);

        Registry {
            entity_counter: 0,
            rooms: HashMap::new(),
            room_codes_by_task: HashMap::new(),
            room_tasks: JoinSet::new(),
            players: HashMap::new(),
            mailbox,
            addr,
        }
    }

    fn generate_name(&mut self, prefix: &str) -> String {
        let id = self.entity_counter;
        self.entity_counter += 1;
        return format!("{prefix}{id:0>5}");
    }

    fn register_player(&mut self) -> String {
        self.generate_name("P")
    }

    fn register_session(&mut self) -> PlayerRegistration {
        let id = self.register_player();
        let resume_token = format!("{:032x}", rand::random::<u128>());
        self.players.insert(
            resume_token.clone(),
            PlayerLease {
                id: id.clone(),
                room: None,
                expires_at: None,
                generation: 0,
            },
        );
        PlayerRegistration {
            id,
            resume_token,
            generation: 0,
        }
    }

    fn reap_expired_players(&mut self) {
        let now = Instant::now();
        self.players
            .retain(|_, player| player.expires_at.is_none_or(|expires_at| expires_at > now));
    }

    fn generate_lobby_code(&self) -> RoomCode {
        loop {
            let candidate = petname::petname(2, "-")
                .expect("the built-in petname dictionaries must contain words");
            if !self.rooms.contains_key(&candidate) {
                return candidate;
            }
        }
    }

    fn register_lobby(&mut self) -> (String, RoomAddr) {
        let code = self.generate_lobby_code();
        let room = Room::new(code.clone());
        let room_handle = room.request_handle();
        let task = self.room_tasks.spawn(room.handle_connection());
        self.room_codes_by_task.insert(task.id(), code.clone());
        self.rooms.insert(code.clone(), room_handle.clone());
        info!(
            target: "multisweeper.registry.room_created",
            room_code = %code,
            room_count = self.rooms.len(),
            "room created"
        );
        (code, room_handle)
    }

    pub fn request_addr(&self) -> RegistryAddr {
        self.addr.clone()
    }

    fn request_lobby(&mut self, code: RoomCode) -> Result<RoomAddr, RegistryError> {
        self.reap_completed_rooms();
        match self.rooms.get(&code) {
            Some(handle) => Ok(handle.clone()),
            None => Err(RegistryError::RoomNotFound(code)),
        }
    }

    fn request_lobbies(&mut self) -> Vec<&RoomCode> {
        self.reap_completed_rooms();
        self.rooms.keys().collect()
    }

    #[tracing::instrument(name = "registry.lifecycle", skip_all)]
    pub async fn handle_connections(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn event_loop(&mut self) -> Result<()> {
        loop {
            self.reap_completed_rooms();
            self.reap_expired_players();
            let event = tokio::select! {
                biased;
                task = self.room_tasks.join_next_with_id(), if !self.room_tasks.is_empty() => RegistryEvent::RoomTask(task),
                msg = self.mailbox.recv() => RegistryEvent::Mailbox(msg),
            };

            match event {
                RegistryEvent::Mailbox(msg) => {
                    let msg = self.receive_mailbox(msg)?;
                    self.handle_mailbox(msg).await?;
                }
                RegistryEvent::RoomTask(Some(task)) => self.handle_room_task(task),
                RegistryEvent::RoomTask(None) => unreachable!("room task set was non-empty"),
            }
        }
    }

    fn reap_completed_rooms(&mut self) {
        while let Some(task) = self.room_tasks.try_join_next_with_id() {
            self.handle_room_task(task);
        }
    }

    fn handle_room_task(&mut self, task: std::result::Result<(TaskId, Result<()>), JoinError>) {
        let task_id = match &task {
            Ok((task_id, _)) => *task_id,
            Err(error) => error.id(),
        };
        let Some(code) = self.room_codes_by_task.remove(&task_id) else {
            error!(
                target: "multisweeper.registry.room_task_untracked",
                task_id = %task_id,
                "completed room task was not registered"
            );
            return;
        };

        let removed = self.rooms.remove(&code);
        if removed.is_none() {
            error!(
                target: "multisweeper.registry.room_untracked",
                room_code = %code,
                "completed room was not advertised"
            );
        }

        match task {
            Ok((_, Ok(()))) => info!(
                target: "multisweeper.registry.room_removed",
                room_code = %code,
                room_count = self.rooms.len(),
                "room removed after task completed"
            ),
            Ok((_, Err(error))) => error!(
                target: "multisweeper.registry.room_failed",
                room_code = %code,
                error = %error,
                room_count = self.rooms.len(),
                "room removed after task failed"
            ),
            Err(error) => error!(
                target: "multisweeper.registry.room_task_failed",
                room_code = %code,
                error = %error,
                room_count = self.rooms.len(),
                "room removed after task terminated unexpectedly"
            ),
        }
    }

    fn receive_mailbox(&mut self, msg: Option<RegistryMessage>) -> Result<RegistryMessage> {
        match msg {
            Some(msg) => Ok(msg),
            None => Err(RegistryError::AddrDropped.into()),
        }
    }

    async fn handle_mailbox(&mut self, msg: RegistryMessage) -> Result<()> {
        let command = registry_message_name(&msg);
        debug!(
            target: "multisweeper.registry.command",
            command,
            "registry command received"
        );
        match msg {
            RegistryMessage::CreateLobby(reply) => {
                let (_code, addr) = self.register_lobby();
                Self::handle_reply(reply, addr).await;
                Ok(())
            }
            RegistryMessage::RequestLobby { code, reply } => {
                let result = self.request_lobby(code);
                if result.is_err() {
                    warn!(
                        target: "multisweeper.registry.room_lookup_failed",
                        error_type = "room_not_found",
                        "room lookup failed"
                    );
                }
                Self::handle_reply(reply, result).await;
                Ok(())
            }
            RegistryMessage::QueryLobbies(reply) => {
                let lobbies = self
                    .request_lobbies()
                    .iter()
                    .map(ToString::to_string)
                    .collect();
                Self::handle_reply(reply, lobbies).await;
                Ok(())
            }
            RegistryMessage::CreatePlayer(reply) => {
                let registration = self.register_session();
                Self::handle_reply(reply, registration).await;
                Ok(())
            }
            RegistryMessage::RegisterPlayerRoom {
                player_id,
                resume_token,
                room,
            } => {
                if let Some(player) = self.players.get_mut(&resume_token) {
                    if player.id == player_id {
                        player.room = Some(room);
                        player.expires_at = None;
                    }
                }
                Ok(())
            }
            RegistryMessage::ClearPlayerRoom { resume_token } => {
                if let Some(player) = self.players.get_mut(&resume_token) {
                    player.room = None;
                    player.expires_at = None;
                }
                Ok(())
            }
            RegistryMessage::DetachPlayer {
                resume_token,
                generation,
            } => {
                if let Some(player) = self.players.get_mut(&resume_token) {
                    if player.generation == generation {
                        player.expires_at = Some(Instant::now() + RESUME_GRACE);
                    }
                }
                Ok(())
            }
            RegistryMessage::ResumePlayer {
                resume_token,
                reply,
            } => {
                self.reap_expired_players();
                let result = match self.players.get_mut(&resume_token) {
                    Some(player) => {
                        player.generation = player.generation.saturating_add(1);
                        player.expires_at = None;
                        Ok(ResumedPlayer {
                            id: player.id.clone(),
                            room: player.room.clone(),
                            generation: player.generation,
                        })
                    }
                    None => Err(RegistryError::PlayerNotFound),
                };
                Self::handle_reply(reply, result).await;
                Ok(())
            }
        }
    }

    async fn handle_reply<T>(reply: ReplyHandle<T>, msg: T) -> () {
        let _ = reply.send(msg);
    }
}

fn registry_message_name(message: &RegistryMessage) -> &'static str {
    match message {
        RegistryMessage::CreateLobby(_) => "create_lobby",
        RegistryMessage::RequestLobby { .. } => "request_lobby",
        RegistryMessage::QueryLobbies(_) => "query_lobbies",
        RegistryMessage::CreatePlayer(_) => "create_player",
        RegistryMessage::RegisterPlayerRoom { .. } => "register_player_room",
        RegistryMessage::ClearPlayerRoom { .. } => "clear_player_room",
        RegistryMessage::DetachPlayer { .. } => "detach_player",
        RegistryMessage::ResumePlayer { .. } => "resume_player",
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{sync::mpsc, time::timeout};

    use super::*;
    use crate::{
        protocol::{
            room::{PlayerCommand, RequestContext, RoomMessage},
            session::SessionMessage,
        },
        session::PlayerId,
    };

    async fn create_lobby(registry_addr: &RegistryAddr) -> RoomAddr {
        let (reply, response) = oneshot::channel();
        registry_addr
            .send(RegistryMessage::CreateLobby(reply))
            .await
            .expect("registry is running");
        response
            .await
            .expect("registry responds with a room address")
    }

    async fn advertised_lobbies(registry_addr: &RegistryAddr) -> Vec<RoomCode> {
        let (reply, response) = oneshot::channel();
        registry_addr
            .send(RegistryMessage::QueryLobbies(reply))
            .await
            .expect("registry is running");
        response
            .await
            .expect("registry responds with advertised rooms")
    }

    #[tokio::test]
    async fn removes_empty_rooms_from_advertised_lobbies() {
        let registry = Registry::new();
        let registry_addr = registry.request_addr();
        let registry_task = tokio::spawn(registry.handle_connections());
        let room_addr = create_lobby(&registry_addr).await;

        assert_eq!(advertised_lobbies(&registry_addr).await.len(), 1);

        let player_id: PlayerId = "player".to_string();
        let (player_addr, mut player_mailbox) = mpsc::channel(1);
        room_addr
            .send(RoomMessage {
                id: player_id.clone(),
                generation: 0,
                request: RequestContext {
                    message_id: "join".to_string(),
                    reply_to: player_addr.clone(),
                },
                command: PlayerCommand::Join {
                    resume_token: "token".to_string(),
                    generation: 0,
                },
            })
            .await
            .expect("room is running");
        timeout(Duration::from_secs(1), player_mailbox.recv())
            .await
            .expect("room acknowledges the join")
            .expect("room sends a join response");

        room_addr
            .send(RoomMessage {
                id: player_id,
                generation: 0,
                request: RequestContext {
                    message_id: "leave".to_string(),
                    reply_to: player_addr,
                },
                command: PlayerCommand::Leave,
            })
            .await
            .expect("room is running");
        let leave_response = timeout(Duration::from_secs(1), player_mailbox.recv())
            .await
            .expect("room acknowledges the leave")
            .expect("room sends a leave response");
        assert!(matches!(
            leave_response,
            SessionMessage::Reply { message, .. }
                if matches!(message, crate::protocol::session::SessionEvent::RoomRemoved { .. })
        ));

        timeout(Duration::from_secs(1), async {
            loop {
                if advertised_lobbies(&registry_addr).await.is_empty() {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("registry removes the completed room");

        registry_task.abort();
        let _ = registry_task.await;
    }
}
