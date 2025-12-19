use std::{io::Error, sync::{Arc}};
use tokio::{net::TcpListener, sync::{Mutex, mpsc}};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};

use tracing::{debug, info, warn};
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

#[tracing::instrument]
async fn handle_connection(stream: tokio::net::TcpStream, state: Arc<Mutex<SharedState>>) {
    let ws_stream = accept_async(stream).await.expect("failed to wrap websocket stream");
    let (mut tx, mut rx) = ws_stream.split();

    let (action_sdr, action_rcr) = mpsc::channel::<ClientMessage>(32);
    let (message_sdr, mut message_rcr) = mpsc::channel::<ServerMessage>(32);

    let player_id = state.lock().await.register_player(action_sdr.clone(), message_sdr);
    let mut player_status = PlayerStatus::Idle { action_rcr };

    loop {
        let result: Option<PlayerStatus> = tokio::select! {
            ws_msg = rx.next() => {
                let Some(Ok(msg)) = ws_msg else {
                    // websocket closed
                    info!("Client {player_id} disconnected");
                    break;
                };
                let bytes = msg.into_data();
                match serde_json::from_slice::<ClientMessage>(&bytes) {
                    Ok(client_msg) => {
                        match client_msg {
                            ClientMessage::CreateLobby => {
                                if let PlayerStatus::Idle { action_rcr } = player_status {
                                    let (cmd_sdr, cmd_rcr) = mpsc::channel::<LobbyCommand>(32);
                                    let host_player = state.lock().await.de_idle_player_by_id(player_id.clone()).unwrap();
                                    let code = state.lock().await.register_lobby(cmd_sdr);
                                    tokio::spawn(lobby_manager_task(cmd_rcr, host_player, action_rcr, code.clone()));
                                    Some(PlayerStatus::Lobby { code })
                                } else {
                                    panic!("illegal operation!");
                                }
                            },
                            ClientMessage::JoinLobby { code } => {
                                if let PlayerStatus::Idle { action_rcr } = player_status {
                                    let (player_id, conn) = state.lock().await.de_idle_player_by_id(player_id.clone()).unwrap();
                                    let state_handle = state.lock().await;
                                    let (code, handle) = state_handle.get_lobby(code).unwrap();
                                    handle.send(LobbyCommand::AddPlayer { id: player_id, player_connection: conn, action_rcr }).await.unwrap();
                                    Some(PlayerStatus::Lobby { code })
                                } else {
                                    panic!("illegal operation!");
                                }
                            },
                            ClientMessage::StartGame => {
                                if let PlayerStatus::Lobby { code } = player_status {
                                    action_sdr.send(ClientMessage::StartGame).await;
                                    Some(PlayerStatus::Game)
                                } else {
                                    panic!("illegal operation!")
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
                let Some(msg) = server_msg else {
                    debug!("Message channel closed for {player_id}");
                    break;
                };
                let Ok(response) = serde_json::to_string(&msg) else { panic!() };
                tx.send(Message::Text(response.into())).await;
                Some(player_status)
            }
        };
        player_status = result.unwrap();
    }
}