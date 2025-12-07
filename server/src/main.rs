use tokio::net::TcpListener;

mod game;
mod cli_local;
mod protocol;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    tracing_forest::init();
    let listener = TcpListener::bind("localhost:8080").await.expect("failed to bind to port");

    while let Ok((stream, _)) = listener.accept().await {

    }
}