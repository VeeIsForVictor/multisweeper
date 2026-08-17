use anyhow::Result;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Error, Message},
};
use tracing::{debug, error, info};

use crate::{
    protocol::{
        registry::RegistryMessage,
        room::{PlayerCommand, RoomMessage},
        session::SessionMessage,
        wire::{ClientRequest, ServerResponse},
    },
    registry::{RegistryAddr, RegistryError},
    room::{RoomAddr, RoomCode},
};

pub type PlayerId = String;
pub type PlayerMailbox = Receiver<SessionMessage>;
pub type PlayerAddr = Sender<SessionMessage>;
pub type PlayerInbound = SplitStream<WebSocketStream<TcpStream>>;
pub type PlayerOutbound = SplitSink<WebSocketStream<TcpStream>, Message>;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("connection was terminated by client")]
    ConnectionTerminated,
    #[error("mailbox dropped")]
    MailboxDropped,
    #[error("handle to room dropped")]
    RoomDropped,
    #[error("already joined a room")]
    RoomAlreadyJoined,
    #[error("no room joined")]
    NoRoomJoined,
}

pub enum SessionEvent {
    Inbound(Option<Result<Message, Error>>),
    Mailbox(Option<SessionMessage>),
}

pub struct Session {
    id: PlayerId,
    mailbox: PlayerMailbox,
    addr: PlayerAddr,
    outbound: PlayerOutbound,
    inbound: PlayerInbound,
    registry_addr: RegistryAddr,
    room: Option<RoomAddr>,
}

impl Session {
    pub fn new(
        id: PlayerId,
        stream: WebSocketStream<TcpStream>,
        registry_addr: RegistryAddr,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let (sink, source) = stream.split();
        Session {
            id,
            mailbox: receiver,
            addr: sender,
            outbound: sink,
            inbound: source,
            registry_addr,
            room: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn request_addr(&self) -> PlayerAddr {
        self.addr.clone()
    }

    #[tracing::instrument(name = "session.lifecycle", skip_all, fields(player_id = %self.id))]
    pub async fn handle_connections(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => (),
            Err(e) => {
                error!(
                    target: "multisweeper.session.failed",
                    player_id = %self.id,
                    error = %e,
                    "session terminated with an error"
                );
                self.terminate().await;
                return Err(e);
            }
        }
        info!(
            target: "multisweeper.session.closed",
            player_id = %self.id,
            "session closed"
        );
        self.terminate().await;
        Ok(())
    }

    async fn event_loop(&mut self) -> Result<()> {
        loop {
            let event = tokio::select! {
                req = self.inbound.next() => SessionEvent::Inbound(req),
                msg = self.mailbox.recv() => SessionEvent::Mailbox(msg)
            };

            match event {
                SessionEvent::Inbound(client_request) => {
                    let request = self.receive_inbound(client_request)?;
                    self.handle_inbound(request).await?;
                }
                SessionEvent::Mailbox(server_message) => {
                    let message = self.receive_mailbox(server_message)?;
                    self.handle_mailbox(message).await?;
                }
            }
        }
    }

    fn receive_inbound(&self, req: Option<Result<Message, Error>>) -> Result<ClientRequest> {
        match req {
            Some(res) => Ok(res?.try_into()?),
            None => Err(SessionError::ConnectionTerminated.into()),
        }
    }

    fn receive_mailbox(&mut self, msg: Option<SessionMessage>) -> Result<SessionMessage> {
        match msg {
            Some(msg) => Ok(msg),
            None => Err(SessionError::RoomDropped.into()),
        }
    }

    #[tracing::instrument(name = "session.request", skip_all, fields(player_id = %self.id))]
    async fn handle_inbound(&mut self, request: ClientRequest) -> Result<()> {
        debug!(
            target: "multisweeper.session.request_received",
            player_id = %self.id,
            request = client_request_name(&request),
            "client request received"
        );
        match request {
            ClientRequest::Ping => Ok(self.send_outbound(ServerResponse::Pong).await?),
            ClientRequest::CreateRoom => {
                if self.room.is_some() {
                    return self
                        .send_outbound(ServerResponse::ClientError(
                            SessionError::RoomAlreadyJoined.to_string(),
                        ))
                        .await;
                }
                let (reply_sdr, reply_rcr) = oneshot::channel::<RoomAddr>();
                self.registry_addr
                    .send(RegistryMessage::CreateLobby(reply_sdr))
                    .await?;
                let addr = reply_rcr.await?;
                self.room = Some(addr);
                Ok(self
                    .send_room(PlayerCommand::Join {
                        handle: self.addr.clone(),
                    })
                    .await?)
            }
            ClientRequest::JoinRoom { room_code } => {
                if self.room.is_some() {
                    return self
                        .send_outbound(ServerResponse::ClientError(
                            SessionError::RoomAlreadyJoined.to_string(),
                        ))
                        .await;
                }
                let (reply_sdr, reply_rcr) = oneshot::channel::<Result<RoomAddr, RegistryError>>();
                self.registry_addr
                    .send(RegistryMessage::RequestLobby {
                        code: room_code,
                        reply: reply_sdr,
                    })
                    .await?;
                let maybe_lobby_handle = reply_rcr.await?;
                match maybe_lobby_handle {
                    Ok(addr) => {
                        self.room = Some(addr);
                        self.send_room(PlayerCommand::Join {
                            handle: self.addr.clone(),
                        })
                        .await?
                    }
                    Err(e) => {
                        self.send_outbound(ServerResponse::ClientError(e.to_string()))
                            .await?
                    }
                }
                Ok(())
            }
            ClientRequest::LeaveRoom => {
                if self.room.is_none() {
                    return self
                        .send_outbound(ServerResponse::ClientError(
                            SessionError::NoRoomJoined.to_string(),
                        ))
                        .await;
                }
                self.send_room(PlayerCommand::Leave).await?;
                Ok(())
            }
            ClientRequest::QueryRooms => {
                let (reply_sdr, reply_rcr) = oneshot::channel::<Vec<RoomCode>>();
                let _ = self
                    .registry_addr
                    .send(RegistryMessage::QueryLobbies(reply_sdr))
                    .await;
                let rooms = reply_rcr.await?;
                Ok(self
                    .send_outbound(ServerResponse::AdvertiseRooms { rooms })
                    .await?)
            }
            ClientRequest::StartGame { difficulty } => {
                let response = match &self.room {
                    None => ServerResponse::ClientError(SessionError::NoRoomJoined.to_string()),
                    Some(room) => {
                        match room
                            .send(RoomMessage {
                                id: self.id.clone(),
                                command: PlayerCommand::StartGame {
                                    difficulty: difficulty.into(),
                                },
                            })
                            .await
                        {
                            Ok(_) => return Ok(()),
                            Err(_) => {
                                ServerResponse::ClientError(SessionError::RoomDropped.to_string())
                            }
                        }
                    }
                };
                self.send_outbound(response).await?;
                Ok(())
            }
            ClientRequest::GameAction { action } => {
                if self.room.is_none() {
                    return self
                        .send_outbound(ServerResponse::ClientError(
                            SessionError::NoRoomJoined.to_string(),
                        ))
                        .await;
                }
                self.send_room(PlayerCommand::GameAction {
                    action: action.into(),
                })
                .await?;
                Ok(())
            }
            ClientRequest::GameQuery => {
                if self.room.is_none() {
                    return self
                        .send_outbound(ServerResponse::ClientError(
                            SessionError::NoRoomJoined.to_string(),
                        ))
                        .await;
                }
                self.send_room(PlayerCommand::GameQuery).await?;
                Ok(())
            }
        }
    }

    #[tracing::instrument(name = "session.mailbox_message", skip_all, fields(player_id = %self.id))]
    async fn handle_mailbox(&mut self, message: SessionMessage) -> Result<()> {
        debug!(
            target: "multisweeper.session.message_received",
            player_id = %self.id,
            message = session_message_name(&message),
            "server message received"
        );
        match message {
            SessionMessage::RoomState {
                code: _,
                owner: _,
                players: _,
                game: _,
            } => {
                self.send_outbound(ServerResponse::Message(message)).await?;
            }
            SessionMessage::Kicked { reason } => {
                self.room = None;
                self.send_outbound(ServerResponse::ClientError(reason))
                    .await?;
            }
            SessionMessage::Error { reason } => {
                self.send_outbound(ServerResponse::ClientError(reason))
                    .await?;
            }
            SessionMessage::GameStarted => {
                self.send_outbound(ServerResponse::Message(message)).await?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "session.send_response", skip_all, fields(player_id = %self.id))]
    async fn send_outbound(&mut self, response: ServerResponse) -> Result<()> {
        debug!(
            target: "multisweeper.session.response_sent",
            player_id = %self.id,
            response = server_response_name(&response),
            "server response sent"
        );
        self.outbound.send(response.try_into()?).await?;
        Ok(())
    }

    #[tracing::instrument(name = "session.send_room_command", skip_all, fields(player_id = %self.id))]
    async fn send_room(&mut self, command: PlayerCommand) -> Result<()> {
        debug!(
            target: "multisweeper.session.room_command_sent",
            player_id = %self.id,
            command = player_command_name(&command),
            "room command sent"
        );
        match &self.room {
            Some(addr) => Ok(addr
                .send(RoomMessage {
                    id: self.id.clone(),
                    command,
                })
                .await?),
            None => Err(SessionError::RoomDropped.into()),
        }
    }

    async fn terminate(mut self) {
        if self.room.is_some() {
            let _ = self.send_room(PlayerCommand::Leave).await;
        }
        let _ = self.outbound.close().await;
    }
}

fn client_request_name(request: &ClientRequest) -> &'static str {
    match request {
        ClientRequest::Ping => "ping",
        ClientRequest::QueryRooms => "query_rooms",
        ClientRequest::JoinRoom { .. } => "join_room",
        ClientRequest::CreateRoom => "create_room",
        ClientRequest::LeaveRoom => "leave_room",
        ClientRequest::StartGame { .. } => "start_game",
        ClientRequest::GameAction { .. } => "game_action",
        ClientRequest::GameQuery => "game_query",
    }
}

fn player_command_name(command: &PlayerCommand) -> &'static str {
    match command {
        PlayerCommand::Join { .. } => "join",
        PlayerCommand::Leave => "leave",
        PlayerCommand::StartGame { .. } => "start_game",
        PlayerCommand::GameAction { .. } => "game_action",
        PlayerCommand::GameQuery => "game_query",
    }
}

fn session_message_name(message: &SessionMessage) -> &'static str {
    match message {
        SessionMessage::RoomState { .. } => "room_state",
        SessionMessage::Kicked { .. } => "kicked",
        SessionMessage::Error { .. } => "error",
        SessionMessage::GameStarted => "game_started",
    }
}

fn server_response_name(response: &ServerResponse) -> &'static str {
    match response {
        ServerResponse::Pong => "pong",
        ServerResponse::AdvertiseRooms { .. } => "advertise_rooms",
        ServerResponse::ClientError(_) => "client_error",
        ServerResponse::Message(_) => "message",
    }
}
