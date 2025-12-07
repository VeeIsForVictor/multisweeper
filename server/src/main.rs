use std::time::SystemTime;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};
use tracing::info;
use crate::protocol::{ClientMessage, ServerMessage};

mod game;
mod cli_local;
mod protocol;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    tracing_forest::init();
    let listener = TcpListener::bind("localhost:8080").await.expect("failed to bind to port");
    println!("WebSocket server is now open at port 8080");

    while let Ok((stream, _)) = listener.accept().await {
        handle_connection(stream).await;
    }
}

#[tracing::instrument(skip_all)]
async fn handle_connection(stream : tokio::net::TcpStream) {
    let ws_stream = accept_async(stream).await.expect("failed to wrap websocket stream");
    let (mut tx, mut rx) = ws_stream.split();

    while let Some(Ok(msg)) = rx.next().await {
        let bytes = msg.into_data();
        match rmp_serde::from_slice::<ClientMessage>(&bytes) {
            Ok(ClientMessage::Ping { time }) => {
                info!(immediate = true, "Received PING at {}", time);
                let time_now = 
                    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("could not get system time");
                let server_data = ServerMessage::Pong { ping_time: time, pong_time: time_now.as_millis()};
                if let Ok(response_data) = rmp_serde::to_vec(&server_data) {
                    tx.send(Message::Binary(response_data.into())).await.expect("failed to send response!");
                }
                info!(immediate = true, "Sent PONG at {}", time_now.as_millis());
            }
            Err(e) => {
                eprintln!("failed to deserialize as client message: {}", e);
                let server_data = ServerMessage::Error(String::from("failed to deserialize as ClientMessage"));
                if let Ok(response_data) = rmp_serde::to_vec(&server_data) {
                    tx.send(Message::Binary(response_data.into())).await.expect("failed to send error response!");
                }
                info!(immediate = true, "Sent ERROR");
            }
        }
    }
}