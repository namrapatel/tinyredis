use crate::{
    cache::{self, Cache},
    resp::RESPMessage,
    simpleElection::{self, *},
};
use anyhow::{Error, Result};
use rand::Rng;
use redis::Client;
use std::env;
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::thread;
use std::{
    collections::btree_map::Keys,
    io::{Read, Write},
    process,
    sync::{Arc, Mutex},
};
use tokio::time::{timeout, Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    stream,
};

const MESSAGE_SIZE: usize = 512;
const CACHE_SIZE: usize = 3;

pub struct Server {
    listener: TcpListener,
    cache: Arc<Mutex<Cache>>,
}

impl Server {
    // Use cargo run <PORT> when starting the server
    // cargo run 6379 key_1 Apple key_2 Orange key_3 Banana
    pub async fn new() -> Result<Self, Error> {
        // get arguments from command line ie. port numbers
        let args: Vec<String> = env::args().collect();
        let PORT = &args[1];

        //gets keys and values from command line
        let mut keys: Vec<String> = Vec::new(); //list of keys
        let mut values: Vec<String> = Vec::new(); //list of values
        for (index, value) in args.iter().enumerate() {
            // iterate over the vector and get a tuple with the index and value of each element
            if index >= 2 && index % 2 == 0 {
                // check if the index is greater than or equal to 2 (i.e. skip the first two arguments, which are the program name and the first argument)
                keys.push(value.clone()); // add the value to the new list
                println!("{:?}", keys)
            } else if index > 2 && index % 2 != 0 {
                values.push(value.clone());
                println!("{:?}", values)
            }
        }

        let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).await?;
        let cache = Arc::new(Mutex::new(Cache::new(CACHE_SIZE)));

        if PORT == "6379" {
            println!("Master Server Started");

            let _ = redis::cmd("SET")
                .arg("key-ttl")
                .arg("value")
                .arg("PX")
                .arg(1000)
                .query::<String>(&mut con)
                .unwrap();

            let length = 10; //set length of replication ID

            let mut rng = rand::thread_rng();
            let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let RUN_ID: String = (0..length)
                .map(|_| rng.gen_range(0..charset.len()))
                .map(|i| charset.chars().nth(i).unwrap())
                .collect();
        } else {
            println!("Replication Server Started");
            let server_addr = SocketAddr::from(([127, 0, 0, 1], 6379)); // connect to the master server
            let mut stream = TcpStream::connect(server_addr).await?;
            let mut buffer = [0; MESSAGE_SIZE];
            // stream.set_read_timeout(Some(Duration::from_secs(1)));

            // let stream_result = TcpStream::connect(server_addr);
            // //check if connection to Master was succesful
            // if let Ok(stream) = stream_result {
            //     println!("Successfully connected to server!");
            //     // Use the `stream` variable to send/receive data
            // } else {
            //     println!("Failed to connect to server!");
            // }

            let bulk_message: RESPMessage = RESPMessage::BulkString("SYNC".to_string());

            let array = RESPMessage::Array(vec![bulk_message]);

            let serialized = array.serialize();

            match stream.write_all(&serialized).await {
                Ok(_) => {}
                Err(e) => println!("Error: {}", e),
                _ => println!("Default"),
            }

            // Read data from the stream into the buffer
            let timeout_duration = Duration::from_secs(5);
            match timeout(timeout_duration, stream.read(&mut buffer)).await {
                Ok(result) => match result {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            println!("No data received from master");
                        } else {
                            // Deserialize the received bytes into a RESPMessage
                            let (response, _) = RESPMessage::deserialize(&buffer);
                            println!("Received response: {:?}", response);
                        }
                    }
                    Err(e) => println!("Error: {}", e),
                },
                Err(_) => println!("Timeout occurred"),
            }

            //TODO: implement sync so replica has all the same data as MASTER
        }

        Ok(Self { listener, cache })
    }

    pub async fn send_set(key: String, value: String, mut stream: TcpStream) -> Result<()> {
        let mut buffer = [0; MESSAGE_SIZE];

        let bulk_message: RESPMessage = RESPMessage::BulkString("SET".to_string());

        let array = RESPMessage::Array(vec![bulk_message]);

        let serialized = array.serialize();

        match stream.write_all(&serialized).await {
            Ok(_) => {}
            Err(e) => println!("Error: {}", e),
            _ => println!("Default"),
        }

        // Read data from the stream into the buffer
        let timeout_duration = Duration::from_secs(5);
        match timeout(timeout_duration, stream.read(&mut buffer)).await {
            Ok(result) => match result {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        //println!("No data received from master");
                        return Err(anyhow::Error::msg("No data received from master"));
                    } else {
                        // Deserialize the received bytes into a RESPMessage
                        let (response, _) = RESPMessage::deserialize(&buffer);
                        println!("Received response: {:?}", response);
                        Ok(())
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            },
            Err(_) => {
                return Err(anyhow::Error::msg("timeout occured"));
            }
        }
    }

    pub async fn run(server: Server) -> Result<()> {
        println!("PROCESS_ID: {}", std::process::id());
        let args: Vec<String> = env::args().collect();
        println!("{:?}", args);
        let PORT = &args[1];

        // spawn thread to handle election stuff
        if PORT != "6379" {
            let handle = thread::spawn(|| {
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
                    String::from("6389"),
                ]);
            });
        }
        //TODO: loop through all replication and forward CACHE Commands to replicas
        loop {
            let incoming = server.listener.accept().await;

            match incoming {
                //check connection
                Ok((mut stream, addr)) => {
                    println!("Handling connection from: {}", addr);
                    let cache = Arc::clone(&server.cache);
                    tokio::spawn(async move {
                        Self::handle_connection(&mut stream, cache).await.unwrap();
                    });
                }
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
                "ping" => RESPMessage::SimpleString("PONG".to_string()),
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
                }
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
                                    let set_result =
                                        cache.set(key.to_string(), value.to_string(), ttl);

                                    match set_result {
                                        Some(_) => Ok(()),
                                        None => Err(()),
                                    }
                                }
                                _ => {
                                    let mut cache = cache.lock().unwrap();
                                    cache.set(key.to_string(), value.to_string(), None);
                                    Ok(())
                                }
                            };
                            match result {
                                Ok(_) => RESPMessage::SimpleString("OK".to_string()),
                                Err(_) => RESPMessage::Error("Error".to_string()),
                            }
                        }
                        _ => RESPMessage::Error("Invalid key or value".to_string()),
                    }
                }
                "getserverid" => {
                    println!("{}", process::id().to_string()); // todo: remove test print
                    RESPMessage::SimpleString(process::id().to_string())
                }
                "setleader" => {
                    println!("Recieved leader message, becoming leader...");
                    // start server on 6379
                    let mut child = Command::new(std::env::args().next().unwrap())
                        .arg("6379")
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .expect("Failed to start new instance of the program");

                    RESPMessage::SimpleString("OK".to_string());
                    process::exit(0);
                }
                "del" => {
                    let key = args.get(0).map(|arg| arg.pack_string());
                    match key {
                        Some(Ok(key)) => match cache.lock().unwrap().remove(key.as_ref()) {
                            Some(value) => RESPMessage::BulkString(value.to_string()),
                            None => RESPMessage::Null,
                        },
                        _ => RESPMessage::Error("Invalid Request".to_string()),
                    }
                }
                "sync" => {
                    //let mut result = Vec::new();
                    println!("TEST");
                    // Acquire the lock on the cache and retrieve all keys
                    let cache = cache.lock().unwrap().get_key();
                    println!("{:?}", cache);
                    RESPMessage::BulkString("PING".to_string())
                    //let mut result: Vec<String> = vec![];

                    //GET keys here
                    /*
                    // Iterate through all keys and retrieve their values
                    for key in keys {
                        if let Some(value) = cache.get(key) {
                            result.push(RESPMessage::BulkString(key.to_string()));
                            result.push(RESPMessage::BulkString(value.clone()));
                        }
                    }
                    // Return a RESPMessage::Array containing all key-value pairs
                    if result.is_empty() {
                        RESPMessage::Null
                    } else {
                        RESPMessage::Array(result)
                    } */
                }
                _ => RESPMessage::Error("Error".to_string()),
            };
            let serialized_response = RESPMessage::serialize(&response);
            match stream.write_all(&serialized_response).await {
                Ok(_) => println!("Write to stream OK"),
                Err(e) => println!("Error {}", e),
                _ => println!("Default catch"),
            };
        }
        Ok(())
    }
}
