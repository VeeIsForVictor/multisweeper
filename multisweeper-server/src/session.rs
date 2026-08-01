use anyhow::Result;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Error, Message},
};

use crate::{
    protocol::{ClientMessage, ClientRequest, PlayerCommand, ServerMessage, ServerResponse}, registry::RegistryHandle, room::RoomHandle,
};

pub type PlayerId = String;
pub type PlayerMailbox = Receiver<ServerMessage>;
pub type PlayerHandle = Sender<ServerMessage>;
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
}

pub enum SessionEvent {
    Inbound(Option<Result<Message, Error>>),
    Mailbox(Option<ServerMessage>),
}

pub struct Session {
    id: PlayerId,
    mailbox: PlayerMailbox,
    handle: PlayerHandle,
    outbound: PlayerOutbound,
    inbound: PlayerInbound,
    registry: RegistryHandle,
    room: Option<RoomHandle>,
}

impl Session {
    pub fn new(id: PlayerId, stream: WebSocketStream<TcpStream>, registry: RegistryHandle) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let (sink, source) = stream.split();
        return Session {
            id,
            mailbox: receiver,
            handle: sender,
            outbound: sink,
            inbound: source,
            registry,
            room: None,
        };
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn request_handle(&self) -> PlayerHandle {
        self.handle.clone()
    }

    pub async fn handle_connections(mut self) -> Result<()> {
        match self.event_loop().await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.terminate();
                Err(e)
            }
        }
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

    fn receive_mailbox(&mut self, msg: Option<ServerMessage>) -> Result<ServerMessage> {
        match msg {
            Some(msg) => Ok(msg),
            None => Err(SessionError::RoomDropped.into()),
        }
    }

    async fn handle_inbound(&mut self, request: ClientRequest) -> Result<()> {
        match request {
            ClientRequest::Ping => Ok(self.send_outbound(ServerResponse::Pong).await?),
            ClientRequest::JoinRoom { room_code } => {
                let maybe_lobby_handle = self.registry.lock().request_lobby(room_code);
                match maybe_lobby_handle {
                    Ok(handle) => {
                        self.room = Some(handle);
                        self.send_room(PlayerCommand::Join { handle: self.handle.clone() }).await?
                    },
                    Err(e) => self.send_outbound(ServerResponse::ClientError(e.to_string())).await?,
                }
                Ok(())
            },
            ClientRequest::LeaveRoom => {
                Ok(self.send_room(PlayerCommand::Leave).await?)
            },
        }
    }

    async fn handle_mailbox(&mut self, message: ServerMessage) -> Result<()> {
        return Ok(());
    }

    async fn send_outbound(&mut self, response: ServerResponse) -> Result<()> {
        return Ok(self.outbound.send(response.try_into()?).await?);
    }

    async fn send_room(&mut self, command: PlayerCommand) -> Result<()> {
        match &self.room {
            Some(handle) => Ok(handle.send(
                ClientMessage {
                    id: self.id.clone(),
                    command
                }
            ).await?),
            None => Err(SessionError::RoomDropped.into()),
        }
    }

    fn terminate(mut self) {
        self.outbound.close();
    }
}
