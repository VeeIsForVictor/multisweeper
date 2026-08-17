use std::println;

use anyhow::Result;
use clap::Parser;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tokio_tungstenite::accept_async;
use tracing::{Instrument, info, warn};
use tracing_subscriber::EnvFilter;

use multisweeper_server::{
    protocol::registry::RegistryMessage,
    registry::{Registry, RegistryAddr},
    session::{PlayerId, Session},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[clap(
        long,
        default_value("8080"),
        help(
            "Set the port to expose the server on, overrides the $SERVER_PORT environment variable (default: 8080)"
        )
    )]
    port: u16,
}

struct Config {
    port: u16,
}

fn read_config() -> Result<Config> {
    let args = Args::parse();
    Ok(Config { port: args.port })
}

#[tracing::instrument]
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let config = read_config()?;
    let registry = Registry::new();
    let registry_addr = registry.request_addr();
    tokio::spawn(registry.handle_connections());

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    println!("server is live and listening on port {}", config.port);
    info!("listening on port {}", config.port);

    while let Ok((stream, addr)) = listener.accept().await {
        let span = tracing::info_span!(
            "connection.accept",
            peer_address = %addr,
        );
        tokio::spawn(accept_connection(stream, registry_addr.clone()).instrument(span));
    }

    warn!("terminating server");

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

#[tracing::instrument(name = "connection.handshake", skip_all)]
async fn accept_connection(stream: TcpStream, registry: RegistryAddr) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (reply_sdr, reply_rcr) = oneshot::channel::<PlayerId>();
    registry
        .send(RegistryMessage::CreatePlayer(reply_sdr))
        .await?;
    let id = reply_rcr.await?;

    info!(
        target: "multisweeper.session.created",
        player_id = %id,
        "session created"
    );
    let session = Session::new(id, ws_stream, registry);
    tokio::spawn(session.handle_connections());
    Ok(())
}
