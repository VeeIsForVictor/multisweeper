use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, stream::StreamExt};

use tracing::{debug, info};
use crate::error::ConnectionError;
use crate::ws::protocol::{ClientMessage, ServerMessage, ErrorCode};
use crate::ws::SharedState;
use crate::ws::{lobby_manager_task, player::{ConnectionState, IdleState, LobbyState}, protocol::{IdleAction, LobbyAction, LobbyCommand}};

mod game;
mod cli_local;
mod error;
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
    let (tx, rx) = ws_stream.split();

    let (action_sdr, action_rcr) = tokio::sync::mpsc::channel::<ClientMessage>(32);
    let (message_sdr, message_rcr) = tokio::sync::mpsc::channel::<ServerMessage>(32);

    let player_id = state.lock().await.register_player(action_sdr.clone(), message_sdr);

    let mut handler = ConnectionHandler::new(
        player_id,
        state,
        tx,
        action_sdr,
        action_rcr,
    );

    handler.run(rx, message_rcr).await;
}

struct ConnectionHandler {
    player_id: String,
    state: Arc<Mutex<SharedState>>,
    tx: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>,
    action_sdr: tokio::sync::mpsc::Sender<ClientMessage>,
    connection_state: ConnectionState,
}

impl ConnectionHandler {
    fn new(
        player_id: String,
        state: Arc<Mutex<SharedState>>,
        tx: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>,
        action_sdr: tokio::sync::mpsc::Sender<ClientMessage>,
        action_rcr: tokio::sync::mpsc::Receiver<ClientMessage>,
    ) -> Self {
        ConnectionHandler {
            player_id,
            state,
            tx,
            action_sdr,
            connection_state: ConnectionState::Idle(IdleState { action_rcr }),
        }
    }

    async fn run(
        &mut self,
        mut rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
        mut message_rcr: tokio::sync::mpsc::Receiver<ServerMessage>,
    ) {
        loop {
            tokio::select! {
                ws_msg = rx.next() => {
                    let Some(Ok(msg)) = ws_msg else {
                        info!("Client {} disconnected", self.player_id);
                        break;
                    };
                    let bytes = msg.into_data();
                    match serde_json::from_slice::<ClientMessage>(&bytes) {
                        Ok(client_msg) => {
                            if let Err(e) = self.handle_client_message(client_msg).await {
                                let code: ErrorCode = e.clone().into();
                                let message = match &e {
                                    ConnectionError::StateTransitionInvalid { from, action } => {
                                        format!("Cannot {} from {}", action, from)
                                    }
                                    ConnectionError::LobbyNotFound => "Lobby not found".to_string(),
                                    ConnectionError::NotHost => "Only the host can perform this action".to_string(),
                                    _ => "An error occurred".to_string(),
                                };
                                self.send_error(code, &message).await;
                            }
                        }
                        Err(e) => {
                            eprintln!("failed to deserialize as client message: {}", e);
                            self.send_error(ErrorCode::DeserializationFailed, "Failed to deserialize message").await;
                            let _ = self.tx.close().await;
                            break;
                        }
                    }
                }

                server_msg = message_rcr.recv() => {
                    let Some(msg) = server_msg else {
                        debug!("Message channel closed for {}", self.player_id);
                        break;
                    };
                    self.forward_server_message(msg).await;
                }
            }
        }

        self.clean_state_on_disconnect().await;
    }

    async fn clean_state_on_disconnect(& mut self) {
        match &self.connection_state {
            ConnectionState::Idle(..) => (),
            ConnectionState::Lobby (LobbyState { code }) => {
                match self.state.lock().await.get_lobby(code.clone()) {
                    None => (),
                    Some((_code, handle)) => {
                        handle.send(LobbyCommand::RemovePlayer { id: self.player_id.clone(), return_to_idle: false }).await;
                    }
                }
            }
            ConnectionState::Game => todo!(),
            ConnectionState::Disconnected => todo!(),
        }
    }

    async fn handle_client_message(&mut self, msg: ClientMessage) -> Result<(), ConnectionError> {
        match msg {
            ClientMessage::IdleClient(action) => self.handle_idle_action(action).await,
            ClientMessage::LobbyClient(action) => self.handle_lobby_action(action).await,
            ClientMessage::GameClient(action) => self.handle_game_action(action).await,
        }
    }

    async fn handle_idle_action(&mut self, action: IdleAction) -> Result<(), ConnectionError> {
        match action {
            IdleAction::CreateLobby => self.handle_create_lobby().await,
            IdleAction::JoinLobby { code } => self.handle_join_lobby(code).await,
        }
    }

    async fn handle_lobby_action(&mut self, action: LobbyAction) -> Result<(), ConnectionError> {
        match action {
            LobbyAction::StartGame => self.handle_start_game().await,
            LobbyAction::LeaveLobby => self.handle_leave_lobby().await,
        }
    }

    async fn handle_game_action(&mut self, _action: crate::ws::protocol::PlayerAction) -> Result<(), ConnectionError> {
        todo!();
    }

    async fn handle_create_lobby(&mut self) -> Result<(), ConnectionError> {
        let player_id = self.player_id.clone();
        let mut state = self.state.lock().await;
        let (player_id, connection) = state.de_idle_player_by_id(player_id)
            .ok_or_else(|| ConnectionError::StateTransitionInvalid {
                from: "Idle".to_string(),
                action: "CreateLobby".to_string(),
            })?;

        let (cmd_sdr, cmd_rcr) = tokio::sync::mpsc::channel::<LobbyCommand>(32);
        let code = state.register_lobby(cmd_sdr);

        let (action_rcr, new_state) = self.connection_state.into_lobby(code.clone())
            .ok_or_else(|| ConnectionError::StateTransitionInvalid {
                from: "Idle".to_string(),
                action: "CreateLobby".to_string(),
            })?;
        self.connection_state = new_state;
        drop(state);

        let state = self.state.clone();
        tokio::spawn(lobby_manager_task(cmd_rcr, (player_id, connection), action_rcr, code, state));
        Ok(())
    }

    async fn handle_join_lobby(&mut self, code: String) -> Result<(), ConnectionError> {
        let player_id = self.player_id.clone();
        let mut state = self.state.lock().await;
        let (player_id, connection) = state.de_idle_player_by_id(player_id)
            .ok_or_else(|| ConnectionError::StateTransitionInvalid {
                from: "Idle".to_string(),
                action: "JoinLobby".to_string(),
            })?;

        if let Some((lobby_code, handle)) = state.get_lobby(code.clone()) {
            let (action_rcr, new_state) = self.connection_state.into_lobby(lobby_code.clone())
                .ok_or_else(|| ConnectionError::StateTransitionInvalid {
                    from: "Idle".to_string(),
                    action: "JoinLobby".to_string(),
                })?;
            self.connection_state = new_state;

            let _ = handle.send(LobbyCommand::AddPlayer {
                id: player_id,
                player_connection: connection,
                action_rcr,
            }).await;
            Ok(())
        } else {
            Err(ConnectionError::LobbyNotFound)
        }
    }

    async fn handle_start_game(&mut self) -> Result<(), ConnectionError> {
        let ConnectionState::Lobby(LobbyState { code: _ }) = &self.connection_state else {
            return Err(ConnectionError::StateTransitionInvalid {
                from: format!("{:?}", self.connection_state),
                action: "StartGame".to_string(),
            });
        };

        let _ = self.action_sdr.send(ClientMessage::LobbyClient(LobbyAction::StartGame)).await;
        self.connection_state = ConnectionState::Game;
        Ok(())
    }

    async fn handle_leave_lobby(&mut self) -> Result<(), ConnectionError> {
        let ConnectionState::Lobby(LobbyState { code }) = &self.connection_state else {
            return Err(ConnectionError::StateTransitionInvalid {
                from: format!("{:?}", self.connection_state),
                action: "LeaveLobby".to_string(),
            });
        };

        let (_new_action_sdr, new_action_rcr) = tokio::sync::mpsc::channel::<ClientMessage>(32);
        let state = self.state.lock().await;
        if let Some((_, handle)) = state.get_lobby(code.clone()) {
            let _ = handle.send(LobbyCommand::RemovePlayer {
                id: self.player_id.clone(),
                return_to_idle: true,
            }).await;
        }
        drop(state);

        self.connection_state = ConnectionState::Idle(IdleState { action_rcr: new_action_rcr });
        Ok(())
    }

    async fn send_error(&mut self, code: ErrorCode, message: &str) {
        let server_data = ServerMessage::Error {
            code,
            message: message.to_string(),
        };
        if let Ok(response_data) = serde_json::to_string(&server_data) {
            let _ = self.tx.send(Message::Text(response_data.into())).await;
        }
    }

    async fn forward_server_message(&mut self, msg: ServerMessage) {
        if let Ok(response) = serde_json::to_string(&msg) {
            let _ = self.tx.send(Message::Text(response.into())).await;
        }
    }
}
