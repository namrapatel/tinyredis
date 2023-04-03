mod server;
mod resp;

use anyhow::{Result};
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    Server::run().await
}