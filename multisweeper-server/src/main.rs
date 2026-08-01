use anyhow::Result;
use clap::Parser;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};
use std::env;

mod protocol;
mod registry;
mod room;
mod session;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[clap(long, help("Set the port to expose the server on, overrides the $SERVER_PORT environment variable"))]
    port: u16
}

struct Config {
    port: u16
}

fn read_config() -> Result<Config> {
    let args = Args::parse();
    let port: u16 = match env::var("SERVER_PORT") {
        Ok(port) => port.parse()?,
        Err(_e) => args.port
    };
    return Ok(Config { port });
}

#[tracing::instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let config = read_config()?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    
    info!("listening on port {}", config.port);

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    warn!("terminating server");

    Ok(())
}

async fn accept_connection(stream: TcpStream) -> Result<()> {

}
