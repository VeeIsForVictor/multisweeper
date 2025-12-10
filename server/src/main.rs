use std::sync::Arc;
use tokio::{net::TcpListener, sync::{RwLock, mpsc}};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};

use ws::{protocol::{ClientMessage, ServerMessage}, SharedState};
use crate::ws::protocol::{LobbyCommand, PlayerConnection};

mod game;
mod cli_local;
mod ws;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    tracing_forest::init();
    let state = Arc::new(RwLock::new(SharedState::new()));
    let listener = TcpListener::bind("localhost:8080").await.expect("failed to bind to port");
    println!("WebSocket server is now open at port 8080");

    while let Ok((stream, _)) = listener.accept().await {
        handle_connection(stream, state.clone()).await;
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, state: Arc<RwLock<SharedState>>) {
    let ws_stream = accept_async(stream).await.expect("failed to wrap websocket stream");
    let (mut tx, mut rx) = ws_stream.split();

    let (mut action_sdr, mut action_rcr) = mpsc::channel::<ClientMessage>(32);
    let (mut message_sdr, mut message_rcr) = mpsc::channel::<ServerMessage>(32);

    state.get_mut().register_player(action_sdr, message_sdr);

    while let Some(Ok(msg)) = rx.next().await {
        let bytes = msg.into_data();
        match serde_json::from_slice::<ClientMessage>(&bytes) {
            Ok(client_msg) => {
                match client_msg {
                    ClientMessage::CreateLobby => {
                        let (cmd_sdr, cmd_rcr) = mpsc::channel::<LobbyCommand>(32);
                        tokio::spawn()
                    },
                    _ => {
                        todo!();
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to deserialize as client message: {}", e);
                let server_data = ServerMessage::Error(String::from("failed to deserialize as ClientMessage"));
                if let Ok(response_data) = serde_json::to_string(&server_data) {
                    tx.send(Message::Text(response_data.into())).await.expect("failed to send error response!");
                }
                // end connection
                tx.close().await.expect("failed to close connection properly");
            }
        }
    }
}