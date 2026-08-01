use anyhow::Result;
use clap::Parser;
use tokio::{net::{TcpListener, TcpStream}};
use tokio_tungstenite::accept_async;
use tracing::{info, warn};
use triomphe::Arc;
use parking_lot::Mutex;

use crate::{registry::{Registry, RegistryHandle}, session::Session};

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
        help("Set the port to expose the server on, overrides the $SERVER_PORT environment variable (default: 8080)")
    )]
    port: u16
}

struct Config {
    port: u16
}

fn read_config() -> Result<Config> {
    let args = Args::parse();
    return Ok(Config { port: args.port });
}

#[tracing::instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let config = read_config()?;
    let registry = Arc::new(Mutex::new(Registry::new()));
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    
    info!("listening on port {}", config.port);

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, registry.clone()));
    }

    warn!("terminating server");

    Ok(())
}

async fn accept_connection(stream: TcpStream, registry: RegistryHandle) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let id = registry.lock().register_player();

    let mut session = Session::new(id, ws_stream, registry);
    tokio::spawn(session.handle_connections());
    return Ok(());
}
