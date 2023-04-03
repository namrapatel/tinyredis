mod server;
mod resp;

use std::io::Result;
use server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    Server::run().await
}