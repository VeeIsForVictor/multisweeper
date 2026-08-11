use asyncapi_rust::AsyncApi;

use super::wire::{ClientRequest, ServerResponse};

#[derive(AsyncApi)]
#[asyncapi(
    title = "Multisweeper WebSocket API",
    version = "0.1.0",
    description = "Real-time room and Minesweeper messages exchanged over WebSocket."
)]
#[asyncapi_server(
    name = "local",
    host = "localhost:8080",
    protocol = "ws",
    description = "Local Multisweeper server"
)]
#[asyncapi_channel(name = "multisweeper", address = "/")]
#[asyncapi_operation(name = "clientMessages", action = "receive", channel = "multisweeper")]
#[asyncapi_operation(name = "serverMessages", action = "send", channel = "multisweeper")]
#[asyncapi_messages(ClientRequest, ServerResponse)]
#[allow(clippy::duplicated_attributes)]
pub struct MultisweeperApi;
