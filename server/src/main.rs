mod game;
mod cli_local;

#[tracing::instrument]
fn main() {
    tracing_forest::init();
}