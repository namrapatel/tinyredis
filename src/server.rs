use anyhow::{Result, Error};
use crate::{resp::RESPMessage, cache::{Cache, self}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use std::{sync::{Arc, Mutex}, collections::HashMap, process};
use std::sync::{Arc, Mutex};
use std::env;

const MESSAGE_SIZE: usize = 512;

pub struct Server {
    listener: TcpListener,
    cache: Arc<Mutex<Cache>>
}

impl Server {

    // Use cargo run <PORT> when starting the server 
    pub async fn new() -> Result<Self, Error> {
        

       //get arguments from command line ie. port numbers 
        let args: Vec<String> = env::args().collect();
        println!("{:?}",args);
        let PORT = &args[1];
        
        let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).await?;
        let cache = Arc::new(Mutex::new(Cache::new()));
    
        Ok(Self { listener, cache })
    }
    
    pub async fn run(server: Server) -> Result<()> {
        loop {
            let incoming = server.listener.accept().await;

            match incoming {
                Ok((mut stream, addr)) => {
                    println!("Handling connection from: {}", addr);
                    let cache = Arc::clone(&server.cache);
                    tokio::spawn(async move {
                        Self::handle_connection(&mut stream, cache).await.unwrap();
                    });
                },
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
    }

    async fn handle_connection(stream: &mut TcpStream, cache: Arc<Mutex<Cache>>) -> Result<()> {
        let mut buffer = [0; MESSAGE_SIZE];

        loop { 
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                println!("Closing connection.");
                break;
            }

            let (message, _) = RESPMessage::deserialize(&buffer);

            let (command, args) = message.to_command()?;
            let response = match command.to_ascii_lowercase().as_ref() {
                "ping" => {
                    RESPMessage::SimpleString("PONG".to_string())
                },
                "echo" => args.first().unwrap().clone(),
                "get" => {
                    let key = args.get(0).map(|arg| arg.pack_string());
    
                    match key {
                        Some(Ok(key)) => match cache.lock().unwrap().get(key.as_ref()) {
                            Some(value) => RESPMessage::BulkString(value),
                            None => RESPMessage::Null,
                        },
                        _ => RESPMessage::Error("Invalid key".to_string()),
                    }
                },
                "set" => {
                    let key = args.get(0).map(|arg| arg.pack_string());
                    let value = args.get(1).map(|arg| arg.pack_string());
                    let px = args.get(3).map(|arg| arg.pack_string());
                
                    match (key, value) {
                        (Some(Ok(key)), Some(Ok(value))) => {
                            let result: Result<(), ()> = match px {
                                Some(Ok(px)) => {
                                    let ttl = px.parse::<u64>().ok().map(|ms| ms / 1000);
                                    let mut cache = cache.lock().unwrap();
                                    cache.set(key.to_string(), value.to_string(), ttl);
                                    Ok(())
                                },
                                _ => {
                                    let mut cache = cache.lock().unwrap();
                                    cache.set(key.to_string(), value.to_string(), None);
                                    Ok(())
                                },
                            };
                            match result {
                                Ok(_) => RESPMessage::SimpleString("OK".to_string()),
                                Err(_) => RESPMessage::Error("Error".to_string()),
                            }
                        },
                        _ => RESPMessage::Error("Invalid key or value".to_string()),
                    }
                },
                "getserverid" => {
                    RESPMessage::SimpleString(process::id().to_string())
                }                                                     
                _ => RESPMessage::Error("Error".to_string())
            };
            let serialized_response = RESPMessage::serialize(&response);
            stream.write_all(&serialized_response).await;
        }
        Ok(())
    }

    // start an election (called when time out after pining leader)
    fn start_election(ports: &[i32]) {
        println!("Server {} is starting an election", process::id());
        // Todo: call helper function to get pid from all bakups

    }

    async fn get_server_ids(ports: &[i32]) -> HashMap<i32, i32> {
        let mut map = HashMap::new();
        // loop over the secondary server ports and get their server Ids
        for port in ports {
            let mut stream = TcpStream::connect("localhost:{port}").await;
            let message = RESPMessage::SimpleString("GETSERVERID".to_string());
            stream.write_all(message).await;
        }
        map
    }


}
