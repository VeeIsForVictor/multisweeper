use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};

use tracing::{debug, info, warn};
use ws::{protocol::{ClientMessage, ServerMessage, ErrorCode}, SharedState};
use crate::ws::{lobby_manager_task, player::{ConnectionState, IdleState, LobbyState}, protocol::{IdleAction, LobbyAction, LobbyCommand}};

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

    let (action_sdr, action_rcr) = tokio::sync::mpsc::channel::<ClientMessage>(32);
    let (message_sdr, mut message_rcr) = tokio::sync::mpsc::channel::<ServerMessage>(32);

    let player_id = state.lock().await.register_player(action_sdr.clone(), message_sdr);
    let mut connection_state = ConnectionState::Idle(IdleState { action_rcr });

    loop {
        let result: Option<ConnectionState> = tokio::select! {
            ws_msg = rx.next() => {
                let Some(Ok(msg)) = ws_msg else {
                    info!("Client {player_id} disconnected");
                    break;
                };
                let bytes = msg.into_data();
                match serde_json::from_slice::<ClientMessage>(&bytes) {
                    Ok(client_msg) => {
                        match client_msg {
                            ClientMessage::IdleClient(IdleAction::CreateLobby) => {
                                if let ConnectionState::Idle(IdleState { action_rcr }) = connection_state {
                                    let (cmd_sdr, cmd_rcr) = tokio::sync::mpsc::channel::<LobbyCommand>(32);
                                    let host_player = state.lock().await.de_idle_player_by_id(player_id.clone()).unwrap();
                                    let code = state.lock().await.register_lobby(cmd_sdr);
                                    tokio::spawn(lobby_manager_task(cmd_rcr, host_player, action_rcr, code.clone()));
                                    Some(ConnectionState::Lobby(LobbyState { code }))
                                } else {
                                    send_error(&mut tx, ErrorCode::InvalidStateTransition, "Cannot create lobby from current state").await;
                                    break;
                                }
                            },
                            ClientMessage::IdleClient(IdleAction::JoinLobby { code }) => {
                                if let ConnectionState::Idle(IdleState { action_rcr }) = connection_state {
                                    let (player_id, conn) = state.lock().await.de_idle_player_by_id(player_id.clone()).unwrap();
                                    let state_handle = state.lock().await;
                                    let (code, handle) = state_handle.get_lobby(code).unwrap();
                                    let _ = handle.send(LobbyCommand::AddPlayer { id: player_id, player_connection: conn, action_rcr }).await;
                                    Some(ConnectionState::Lobby(LobbyState { code }))
                                } else {
                                    send_error(&mut tx, ErrorCode::InvalidStateTransition, "Cannot join lobby from current state").await;
                                    break;
                                }
                            },
                            ClientMessage::LobbyClient(LobbyAction::StartGame) => {
                                if let ConnectionState::Lobby(LobbyState { code: _ }) = connection_state {
                                    let _ = action_sdr.send(client_msg).await;
                                    Some(ConnectionState::Game)
                                } else {
                                    send_error(&mut tx, ErrorCode::InvalidStateTransition, "Cannot start game from current state").await;
                                    break;
                                }
                            }
                            _ => {
                                todo!();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to deserialize as client message: {}", e);
                        let server_data = ServerMessage::Error {
                            code: ErrorCode::DeserializationFailed,
                            message: "Failed to deserialize message".to_string()
                        };
                        if let Ok(response_data) = serde_json::to_string(&server_data) {
                            let _ = tx.send(Message::Text(response_data.into())).await;
                        }
                        let _ = tx.close().await;
                        break;
                    }
                }
            },

            server_msg = message_rcr.recv() => {
                let Some(msg) = server_msg else {
                    debug!("Message channel closed for {player_id}");
                    break;
                };
                let Ok(response) = serde_json::to_string(&msg) else { panic!() };
                let _ = tx.send(Message::Text(response.into())).await;
                Some(connection_state)
            }
        };
        connection_state = result.unwrap();
    }
}

async fn send_error(tx: &mut (impl SinkExt<Message> + Unpin), code: ErrorCode, message: &str) {
    let server_data = ServerMessage::Error {
        code,
        message: message.to_string()
    };
    if let Ok(response_data) = serde_json::to_string(&server_data) {
        let _ = tx.send(Message::Text(response_data.into())).await;
    }
}
