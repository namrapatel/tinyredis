mod server;
mod resp;
mod cache;
mod simpleElection;

use anyhow::{Result};
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::new().await?;

    Server::run(server).await?;
    Ok(())
}