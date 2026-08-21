use std::collections::HashSet;

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
        room::{PlayerCommand, RequestContext, RoomMessage},
        session::{ClientError, ErrorCode, MessageId, SessionMessage},
        wire::{ClientRequest, ServerMessage},
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

impl SessionError {
    fn client_error(&self) -> ClientError {
        let code = match self {
            Self::ConnectionTerminated => ErrorCode::RoomUnavailable,
            Self::MailboxDropped | Self::RoomDropped => ErrorCode::RoomDropped,
            Self::RoomAlreadyJoined => ErrorCode::RoomAlreadyJoined,
            Self::NoRoomJoined => ErrorCode::NoRoomJoined,
        };
        ClientError::new(code, self.to_string())
    }
}

enum InboundError {
    ConnectionTerminated,
    Transport(Error),
    Malformed(serde_json::Error),
    UnsupportedFrame,
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
    seen_message_ids: HashSet<MessageId>,
    message_counter: u64,
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
            seen_message_ids: HashSet::new(),
            message_counter: 0,
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
        let message_id = self.next_message_id();
        self.send_outbound(ServerMessage::ConnectionReady {
            message_id,
            player_id: self.id.clone(),
        })
        .await?;
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
                    match self.receive_inbound(client_request) {
                        Ok(request) => {
                            let message_id = request.message_id().clone();
                            if let Err(error) = self.accept_message_id(&message_id) {
                                self.send_rejection(Some(message_id), error).await?;
                            } else {
                                self.handle_inbound(request).await?;
                            }
                        }
                        Err(InboundError::Malformed(error)) => {
                            self.send_rejection(
                                None,
                                ClientError::new(
                                    ErrorCode::InvalidMessage,
                                    format!("invalid client message: {error}"),
                                ),
                            )
                            .await?;
                        }
                        Err(InboundError::UnsupportedFrame) => {
                            self.send_rejection(
                                None,
                                ClientError::new(
                                    ErrorCode::InvalidMessage,
                                    "unsupported WebSocket frame for the application protocol",
                                ),
                            )
                            .await?;
                        }
                        Err(InboundError::ConnectionTerminated) => {
                            return Err(SessionError::ConnectionTerminated.into());
                        }
                        Err(InboundError::Transport(error)) => return Err(error.into()),
                    }
                }
                SessionEvent::Mailbox(server_message) => {
                    let message = self.receive_mailbox(server_message)?;
                    self.handle_mailbox(message).await?;
                }
            }
        }
    }

    fn receive_inbound(
        &self,
        req: Option<Result<Message, Error>>,
    ) -> Result<ClientRequest, InboundError> {
        match req {
            Some(Ok(message @ (Message::Text(_) | Message::Binary(_)))) => {
                message.try_into().map_err(InboundError::Malformed)
            }
            Some(Ok(Message::Close(_))) | None => Err(InboundError::ConnectionTerminated),
            Some(Ok(_)) => Err(InboundError::UnsupportedFrame),
            Some(Err(error)) => Err(InboundError::Transport(error)),
        }
    }

    fn accept_message_id(&mut self, message_id: &MessageId) -> Result<(), ClientError> {
        if message_id.is_empty() || message_id.len() > 128 {
            return Err(ClientError::new(
                ErrorCode::InvalidMessage,
                "message_id must contain between 1 and 128 bytes",
            ));
        }
        if !self.seen_message_ids.insert(message_id.clone()) {
            return Err(ClientError::new(
                ErrorCode::DuplicateMessageId,
                "message_id was already used on this connection",
            ));
        }
        Ok(())
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
            ClientRequest::ConnectionPing { message_id } => {
                let response_message_id = self.next_message_id();
                Ok(self
                    .send_outbound(ServerMessage::ConnectionPong {
                        message_id: response_message_id,
                        correlation_id: message_id,
                    })
                    .await?)
            }
            ClientRequest::RoomCreate { message_id } => {
                if self.room.is_some() {
                    return self
                        .send_rejection(
                            Some(message_id),
                            SessionError::RoomAlreadyJoined.client_error(),
                        )
                        .await;
                }
                let (reply_sdr, reply_rcr) = oneshot::channel::<RoomAddr>();
                if self
                    .registry_addr
                    .send(RegistryMessage::CreateLobby(reply_sdr))
                    .await
                    .is_err()
                {
                    return self
                        .send_rejection(Some(message_id), SessionError::RoomDropped.client_error())
                        .await;
                }
                let addr = match reply_rcr.await {
                    Ok(addr) => addr,
                    Err(_) => {
                        return self
                            .send_rejection(
                                Some(message_id),
                                SessionError::RoomDropped.client_error(),
                            )
                            .await;
                    }
                };
                self.room = Some(addr);
                self.send_room_or_reject(message_id, PlayerCommand::Join)
                    .await
            }
            ClientRequest::RoomJoin {
                message_id,
                room_code,
            } => {
                if self.room.is_some() {
                    return self
                        .send_rejection(
                            Some(message_id),
                            SessionError::RoomAlreadyJoined.client_error(),
                        )
                        .await;
                }
                let (reply_sdr, reply_rcr) = oneshot::channel::<Result<RoomAddr, RegistryError>>();
                if self
                    .registry_addr
                    .send(RegistryMessage::RequestLobby {
                        code: room_code,
                        reply: reply_sdr,
                    })
                    .await
                    .is_err()
                {
                    return self
                        .send_rejection(Some(message_id), SessionError::RoomDropped.client_error())
                        .await;
                }
                let maybe_lobby_handle = match reply_rcr.await {
                    Ok(result) => result,
                    Err(_) => {
                        return self
                            .send_rejection(
                                Some(message_id),
                                SessionError::RoomDropped.client_error(),
                            )
                            .await;
                    }
                };
                match maybe_lobby_handle {
                    Ok(addr) => {
                        self.room = Some(addr);
                        self.send_room_or_reject(message_id, PlayerCommand::Join)
                            .await?
                    }
                    Err(e) => {
                        let error = match e {
                            RegistryError::RoomNotFound(code) => ClientError::new(
                                ErrorCode::RoomNotFound,
                                format!("requested room with code {code} not found"),
                            ),
                            RegistryError::AddrDropped | RegistryError::ReplyFailed => {
                                SessionError::RoomDropped.client_error()
                            }
                        };
                        self.send_rejection(Some(message_id), error).await?
                    }
                }
                Ok(())
            }
            ClientRequest::RoomLeave { message_id } => {
                if self.room.is_none() {
                    return self
                        .send_rejection(Some(message_id), SessionError::NoRoomJoined.client_error())
                        .await;
                }
                self.send_room_or_reject(message_id, PlayerCommand::Leave)
                    .await
            }
            ClientRequest::RoomsList { message_id } => {
                let (reply_sdr, reply_rcr) = oneshot::channel::<Vec<RoomCode>>();
                if self
                    .registry_addr
                    .send(RegistryMessage::QueryLobbies(reply_sdr))
                    .await
                    .is_err()
                {
                    return self
                        .send_rejection(Some(message_id), SessionError::RoomDropped.client_error())
                        .await;
                }
                let rooms = match reply_rcr.await {
                    Ok(rooms) => rooms,
                    Err(_) => {
                        return self
                            .send_rejection(
                                Some(message_id),
                                SessionError::RoomDropped.client_error(),
                            )
                            .await;
                    }
                };
                let response_message_id = self.next_message_id();
                Ok(self
                    .send_outbound(ServerMessage::RoomsListed {
                        message_id: response_message_id,
                        correlation_id: message_id,
                        rooms,
                    })
                    .await?)
            }
            ClientRequest::GameStart {
                message_id,
                difficulty,
            } => {
                if self.room.is_none() {
                    return self
                        .send_rejection(Some(message_id), SessionError::NoRoomJoined.client_error())
                        .await;
                }
                self.send_room_or_reject(
                    message_id,
                    PlayerCommand::StartGame {
                        difficulty: difficulty.into(),
                    },
                )
                .await
            }
            ClientRequest::GameAction {
                message_id,
                action,
                x,
                y,
            } => {
                if self.room.is_none() {
                    return self
                        .send_rejection(Some(message_id), SessionError::NoRoomJoined.client_error())
                        .await;
                }
                self.send_room_or_reject(
                    message_id,
                    PlayerCommand::GameAction {
                        action: action.into_game_action(x, y),
                    },
                )
                .await
            }
            ClientRequest::RoomStateGet { message_id } => {
                if self.room.is_none() {
                    return self
                        .send_rejection(Some(message_id), SessionError::NoRoomJoined.client_error())
                        .await;
                }
                self.send_room_or_reject(message_id, PlayerCommand::GameQuery)
                    .await
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
        if matches!(
            &message,
            SessionMessage::Reply {
                message: crate::protocol::session::SessionEvent::RoomRemoved { .. }
                    | crate::protocol::session::SessionEvent::RoomJoinRejected { .. },
                ..
            } | SessionMessage::Broadcast(
                crate::protocol::session::SessionEvent::RoomRemoved { .. },
            )
        ) {
            self.room = None;
        }
        let response_message_id = self.next_message_id();
        self.send_outbound(ServerMessage::from_session(response_message_id, message))
            .await?;

        Ok(())
    }

    async fn send_rejection(
        &mut self,
        correlation_id: Option<MessageId>,
        error: ClientError,
    ) -> Result<()> {
        let response_message_id = self.next_message_id();
        self.send_outbound(ServerMessage::CommandRejected {
            message_id: response_message_id,
            correlation_id,
            error,
        })
        .await
    }

    #[tracing::instrument(name = "session.send_response", skip_all, fields(player_id = %self.id))]
    async fn send_outbound(&mut self, response: ServerMessage) -> Result<()> {
        debug!(
            target: "multisweeper.session.response_sent",
            player_id = %self.id,
            response = server_message_name(&response),
            "server response sent"
        );
        self.outbound.send(response.try_into()?).await?;
        Ok(())
    }

    fn next_message_id(&mut self) -> MessageId {
        self.message_counter += 1;
        format!("s-{}-{:016x}", self.id, self.message_counter)
    }

    #[tracing::instrument(name = "session.send_room_command", skip_all, fields(player_id = %self.id))]
    async fn send_room(&mut self, message_id: MessageId, command: PlayerCommand) -> Result<()> {
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
                    request: RequestContext {
                        message_id,
                        reply_to: self.addr.clone(),
                    },
                    command,
                })
                .await?),
            None => Err(SessionError::RoomDropped.into()),
        }
    }

    async fn send_room_or_reject(
        &mut self,
        message_id: MessageId,
        command: PlayerCommand,
    ) -> Result<()> {
        match self.send_room(message_id.clone(), command).await {
            Ok(()) => Ok(()),
            Err(_) => {
                self.room = None;
                self.send_rejection(Some(message_id), SessionError::RoomDropped.client_error())
                    .await
            }
        }
    }

    async fn terminate(mut self) {
        if self.room.is_some() {
            let message_id = self.next_message_id();
            let _ = self.send_room(message_id, PlayerCommand::Leave).await;
        }
        let _ = self.outbound.close().await;
    }
}

fn client_request_name(request: &ClientRequest) -> &'static str {
    match request {
        ClientRequest::ConnectionPing { .. } => "connection_ping",
        ClientRequest::RoomsList { .. } => "rooms_list",
        ClientRequest::RoomJoin { .. } => "room_join",
        ClientRequest::RoomCreate { .. } => "room_create",
        ClientRequest::RoomLeave { .. } => "room_leave",
        ClientRequest::GameStart { .. } => "game_start",
        ClientRequest::GameAction { .. } => "game_action",
        ClientRequest::RoomStateGet { .. } => "room_state_get",
    }
}

fn player_command_name(command: &PlayerCommand) -> &'static str {
    match command {
        PlayerCommand::Join => "join",
        PlayerCommand::Leave => "leave",
        PlayerCommand::StartGame { .. } => "start_game",
        PlayerCommand::GameAction { .. } => "game_action",
        PlayerCommand::GameQuery => "game_query",
    }
}

fn session_message_name(message: &SessionMessage) -> &'static str {
    match message {
        SessionMessage::Reply { message, .. } | SessionMessage::Broadcast(message) => match message
        {
            crate::protocol::session::SessionEvent::RoomState { .. } => "room_state",
            crate::protocol::session::SessionEvent::RoomRemoved { .. } => "room_removed",
            crate::protocol::session::SessionEvent::RoomJoinRejected { .. } => "room_join_rejected",
            crate::protocol::session::SessionEvent::Error { .. } => "error",
            crate::protocol::session::SessionEvent::GameStarted => "game_started",
        },
    }
}

fn server_message_name(response: &ServerMessage) -> &'static str {
    match response {
        ServerMessage::ConnectionReady { .. } => "connection_ready",
        ServerMessage::ConnectionPong { .. } => "connection_pong",
        ServerMessage::RoomsListed { .. } => "rooms_listed",
        ServerMessage::RoomState { .. } => "room_state",
        ServerMessage::RoomRemoved { .. } => "room_removed",
        ServerMessage::CommandRejected { .. } => "command_rejected",
        ServerMessage::GameStarted { .. } => "game_started",
    }
}
