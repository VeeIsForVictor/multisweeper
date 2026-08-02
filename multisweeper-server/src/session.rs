use anyhow::Result;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use thiserror::Error;
use tokio::{
    net::TcpStream, sync::{mpsc::{self, Receiver, Sender}, oneshot},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Error, Message},
};

use crate::{
    protocol::{ClientMessage, ClientRequest, PlayerCommand, RegistryMessage::{self, RequestLobby}, ServerMessage, ServerResponse}, registry::{RegistryAddr, RegistryError}, room::{RoomAddr, RoomCode},
};

pub type PlayerId = String;
pub type PlayerMailbox = Receiver<ServerMessage>;
pub type PlayerAddr = Sender<ServerMessage>;
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
    RoomAlreadyJoined
}

pub enum SessionEvent {
    Inbound(Option<Result<Message, Error>>),
    Mailbox(Option<ServerMessage>),
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
    pub fn new(id: PlayerId, stream: WebSocketStream<TcpStream>, registry_addr: RegistryAddr) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let (sink, source) = stream.split();
        return Session {
            id,
            mailbox: receiver,
            addr: sender,
            outbound: sink,
            inbound: source,
            registry_addr,
            room: None,
        };
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn request_addr(&self) -> PlayerAddr {
        self.addr.clone()
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
            ClientRequest::CreateRoom => {
                let (reply_sdr, reply_rcr) = oneshot::channel::<RoomAddr>();
                self.registry_addr.send(RegistryMessage::CreateLobby(reply_sdr)).await?;
                let addr = reply_rcr.blocking_recv()?;
                self.room = Some(addr);
                Ok(self.send_room(PlayerCommand::Join { handle: self.addr.clone() }).await?)
            }
            ClientRequest::JoinRoom { room_code } => {
                match &self.room {
                    Some(_handle) => return Ok(self.send_outbound(ServerResponse::ClientError(SessionError::RoomAlreadyJoined.to_string())).await?),
                    None => (),
                };
                let (reply_sdr, reply_rcr) = oneshot::channel::<Result<RoomAddr, RegistryError>>();
                self.registry_addr.send(RegistryMessage::RequestLobby { code: room_code, reply: reply_sdr }).await?;
                let maybe_lobby_handle = reply_rcr.blocking_recv()?;
                match maybe_lobby_handle {
                    Ok(addr) => {
                        self.room = Some(addr);
                        self.send_room(PlayerCommand::Join { handle: self.addr.clone() }).await?
                    },
                    Err(e) => self.send_outbound(ServerResponse::ClientError(e.to_string())).await?,
                }
                Ok(())
            },
            ClientRequest::LeaveRoom => {
                Ok(self.send_room(PlayerCommand::Leave).await?)
            },
            ClientRequest::QueryRooms => {
                let (reply_sdr, reply_rcr) = oneshot::channel::<Vec<RoomCode>>();
                self.registry_addr.send(RegistryMessage::QueryLobbies(reply_sdr));
                let rooms = reply_rcr.blocking_recv()?;
                Ok(self.send_outbound(ServerResponse::AdvertiseRooms { rooms }).await?)
            },
            ClientRequest::StartGame => todo!(),
            ClientRequest::GameAction => todo!(),
            ClientRequest::GameQuery => todo!(),
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
            Some(addr) => Ok(addr.send(
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
