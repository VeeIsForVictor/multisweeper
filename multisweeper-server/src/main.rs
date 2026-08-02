use std::println;

use anyhow::Result;
use clap::Parser;
use tokio::{net::{TcpListener, TcpStream}, sync::oneshot};
use tokio_tungstenite::accept_async;
use tracing::{info, warn};

use crate::{
    protocol::RegistryMessage, registry::{Registry, RegistryAddr}, session::{PlayerId, Session},
};

mod protocol;
mod registry;
mod room;
mod session;

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
    return Ok(Config { port: args.port });
}

#[tracing::instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let config = read_config()?;
    let registry = Registry::new();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;

    println!("server is live and listening on port {}", config.port);
    info!("listening on port {}", config.port);

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, registry.request_addr()));
    }

    warn!("terminating server");

    Ok(())
}

async fn accept_connection(stream: TcpStream, registry: RegistryAddr) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (reply_sdr, reply_rcr) = oneshot::channel::<PlayerId>();
    registry.send(RegistryMessage::CreatePlayer(reply_sdr)).await?;
    let id = reply_rcr.blocking_recv()?;

    let session = Session::new(id, ws_stream, registry);
    tokio::spawn(session.handle_connections());
    return Ok(());
}
