use futures::{StreamExt, stream::{SplitSink, SplitStream}};
use tokio::{io::split, net::TcpStream, sync::mpsc::{self, Receiver, Sender}};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use crate::protocol::{ServerMessage, ServerResponse};

pub type PlayerId = String;
pub type PlayerMailbox = Receiver<ServerMessage>;
pub type PlayerHandle = Sender<ServerMessage>;
pub type PlayerInbound = SplitStream<WebSocketStream<TcpStream>>;
pub type PlayerOutbound = SplitSink<WebSocketStream<TcpStream>, Message>;

pub struct Session {
    id: PlayerId,
    mailbox: PlayerMailbox,
    handle: PlayerHandle,
    outbound: PlayerOutbound,
    inbound: PlayerInbound
}

impl Session {
    pub fn new(id: PlayerId, stream: WebSocketStream<TcpStream>) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let (sink, source) = stream.split();
        return Session { id, mailbox: receiver, handle: sender, outbound: sink, inbound: source };
    }
}