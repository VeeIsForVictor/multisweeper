use std::{collections::HashMap, sync::Arc, time::SystemTime};
use tokio::{net::TcpListener, sync::RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};
use tracing::info;

use ws::{protocol::{ClientMessage, ServerMessage},SharedState};

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

    while let Some(Ok(msg)) = rx.next().await {
        let bytes = msg.into_data();
        match serde_json::from_slice::<ClientMessage>(&bytes) {
            Ok(client_msg) => {
                
            }
            Err(e) => {
                eprintln!("failed to deserialize as client message: {}", e);
                let server_data = ServerMessage::Error(String::from("failed to deserialize as ClientMessage"));
                if let Ok(response_data) = serde_json::to_string(&server_data) {
                    tx.send(Message::Text(response_data.into())).await.expect("failed to send error response!");
                }
                info!(immediate = true, "Sent ERROR");
            }
        }
    }
}