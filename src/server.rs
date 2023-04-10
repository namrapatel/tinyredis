use anyhow::{Result, Error};
use crate::{resp::RESPMessage, cache::{Cache, self}, simpleElection::{*, self}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use std::{sync::{Arc, Mutex}, process};
use std::env;
use std::thread;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const MESSAGE_SIZE: usize = 512;

pub struct Server {
    listener: TcpListener,
    cache: Arc<Mutex<Cache>>,
    should_run: Arc<AtomicBool>, // Add a flag to indicate whether the server should continue running
}

impl Server {
    // Use cargo run <PORT> when starting the server 
    pub async fn new() -> Result<Self, Error> {
    
       //get arguments from command line ie. port numbers 
        let args: Vec<String> = env::args().collect();
        let PORT = &args[1];
        
        let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).await?;
        let cache = Arc::new(Mutex::new(Cache::new()));
        let should_run = Arc::new(AtomicBool::new(true));
        Ok(Self { listener, cache, should_run })
    }

    // Getter method to access the `cache` field
    pub fn get_cache(&self) -> Arc<Mutex<Cache>> {
        Arc::clone(&self.cache)
    }

    // Getter method to access the `should_run` field
    pub fn should_run(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.should_run)
    }

    // New method to create a server instance with a specified port and an existing cache
    pub async fn with_port_and_cache(port: u16, cache: Arc<Mutex<Cache>>, should_run: Arc<AtomicBool>) -> Result<Self, Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        // Initialize the `should_run` field
        Ok(Self { listener, cache, should_run })
    }
    
    pub async fn run(server: Server) -> Result<()> {  
        println!("PROCESS_ID: {}", std::process::id());
        let args: Vec<String> = env::args().collect();
        println!("{:?}",args);
        let PORT = &args[1];

        // spawn thread to handle election stuff
        if PORT != "6379" {
            let handle =thread::spawn(|| {
                // pass a list of potential port numbers that backups can be on 
                // ping leader will call an election using these ports if a pong is not
                // recieved from the leader in 10 seconds
                simpleElection::ping_leader(&vec![
                    String::from("6380"), 
                    String::from("6381"),
                    String::from("6382"),
                    String::from("6383"),
                    String::from("6384"),
                    String::from("6385"),
                    String::from("6386"),
                    String::from("6387"),
                    String::from("6388"),
                    String::from("6389")
                    ]);
            });
        }  

        loop {
            if !server.should_run().load(Ordering::Relaxed) {
                break;
            }

            let incoming = server.listener.accept().await;

            match incoming {
                Ok((mut stream, addr)) => {
                    println!("Handling connection from: {}", addr);
                    let cache = Arc::clone(&server.cache);
                    let should_run = Arc::clone(&server.should_run); // Create a clone of the should_run flag
                    tokio::spawn(async move {
                        Self::handle_connection(&mut stream, cache, should_run).await.unwrap();
                    });
                },
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
        Ok(())
    }

    async fn handle_connection(
        stream: &mut TcpStream, 
        cache: Arc<Mutex<Cache>>,
        should_run: Arc<AtomicBool>,
    ) -> Result<()> {
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
                    println!("{}", process::id().to_string()); // todo: remove test print
                    RESPMessage::SimpleString(process::id().to_string())
                },
                "setleader" => {
                    println!("Recieved leader message, becoming leader...");
                    // start server on 6379
                    // let mut child = Command::new(std::env::args().next().unwrap())
                    //     .arg("6379")
                    //     .stdin(Stdio::null())
                    //     .stdout(Stdio::null())
                    //     .stderr(Stdio::null())
                    //     .spawn()
                    //     .expect("Failed to start new instance of the program");

                    // Set the flag to stop the current server instance
                    should_run.store(false, Ordering::Relaxed);
                    RESPMessage::SimpleString("OK".to_string())
                    // process::exit(0);
                }                                            
                _ => RESPMessage::Error("Error".to_string())
            };
            let serialized_response = RESPMessage::serialize(&response);
            match stream.write_all(&serialized_response).await {
                Ok(_) => println!("write to stream OK"),
                Err(e) => println!("Error {}", e),
                _ => println!("Default catch"),
            };
        }
        Ok(())
    }
}
