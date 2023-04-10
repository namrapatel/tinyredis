mod server;
mod resp;
mod cache;
mod simpleElection;

use anyhow::{Result};
use server::Server;
use cache::Cache;
use std::{sync::{Arc, Mutex}, env};
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() {
    // Get the port number from the command-line arguments
    let args: Vec<String> = env::args().collect();
    let initial_port = args.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(6379);

    let mut port = initial_port; // Initial port number
    let cache = Arc::new(Mutex::new(Cache::new())); // Initial cache

    loop {
        let should_run = Arc::new(AtomicBool::new(true));
        let server = Server::with_port_and_cache(port, Arc::clone(&cache), Arc::clone(&should_run)).await.unwrap();
        Server::run(server).await.unwrap();

        // Check if the server stopped due to the "setleader" command
        if !should_run.load(Ordering::Relaxed) {
            // If the server stopped because of the "setleader" command, start a new server on port 6379
            port = 6379;
            // Print the cache before starting the new server instance
            let cache_lock = cache.lock().unwrap();
            println!("Cache contents: {:?}", *cache_lock);
        } else {
            // If the server stopped for other reasons, break the loop and exit
            break;
        }
    }
}