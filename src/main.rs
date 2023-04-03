mod server;
mod resp;
mod cache;

use anyhow::{Result};
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    Server::run().await
}