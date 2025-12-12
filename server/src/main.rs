use std::{io::Error, sync::{Arc, Mutex}};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};

use tracing::warn;
use ws::{protocol::{ClientMessage, ServerMessage}, SharedState};
use crate::ws::{lobby_manager_task, player::PlayerStatus, protocol::LobbyCommand};

mod game;
mod cli_local;
mod ws;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    tracing_forest::init();
    let state = Arc::new(Mutex::new(SharedState::new(69)));
    let listener = TcpListener::bind("localhost:8080").await.expect("failed to bind to port");
    println!("WebSocket server is now open at port 8080");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, state.clone()));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, state: Arc<Mutex<SharedState>>) {
    let ws_stream = accept_async(stream).await.expect("failed to wrap websocket stream");
    let (mut tx, mut rx) = ws_stream.split();

    let (action_sdr, action_rcr) = mpsc::channel::<ClientMessage>(32);
    let (message_sdr, mut message_rcr) = mpsc::channel::<ServerMessage>(32);

    let player_id = state.lock().expect("poisoned lock").register_player(action_sdr, message_sdr);
    let mut player_status = PlayerStatus::Idle { action_rcr };

    loop {
        let result: Option<PlayerStatus> = tokio::select! {
            ws_msg = rx.next() => {
                let Some(Ok(msg)) = ws_msg else {panic!()};
                let bytes = msg.into_data();
                match serde_json::from_slice::<ClientMessage>(&bytes) {
                    Ok(client_msg) => {
                        match client_msg {
                            ClientMessage::CreateLobby => {
                                if let PlayerStatus::Idle { action_rcr } = player_status {
                                    let (cmd_sdr, cmd_rcr) = mpsc::channel::<LobbyCommand>(32);
                                    let host_player = state.lock().expect("poisoned lock").de_idle_player_by_id(player_id.clone()).unwrap();
                                    let code = state.lock().expect("poisoned lock").register_lobby(cmd_sdr);
                                    tokio::spawn(lobby_manager_task(cmd_rcr, host_player, action_rcr, code.clone()));
                                    Some(PlayerStatus::Lobby { code })
                                } else {
                                    panic!("illegal operation!");
                                }
                            },
                            ClientMessage::JoinLobby { code } => {
                                if let PlayerStatus::Idle { action_rcr } = player_status {
                                    let mut state = state.lock().expect("poisoned lock");
                                    let (code, handle) = state.get_lobby(code).unwrap();
                                    let (player_id, conn) = state.de_idle_player_by_id(player_id.clone()).unwrap();
                                    handle.send(LobbyCommand::AddPlayer { id: player_id, player_connection: conn, action_rcr }).await.unwrap();
                                    Some(PlayerStatus::Lobby { code })
                                } else {
                                    panic!("illegal operation!");
                                }
                            }
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
                        None
                    }
                }
            },
    
            server_msg = message_rcr.recv() => {
                let Some(msg) = server_msg else { panic!() };
                let Ok(response) = serde_json::to_string(&msg) else { panic!() };
                tx.send(Message::Text(response.into())).await;
                None
            }
        };
        player_status = result.unwrap();
    }
}