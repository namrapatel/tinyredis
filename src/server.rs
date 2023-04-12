use crate::{
    cache::{self, Cache},
    resp::RESPMessage,
    simpleElection::{self, *},
};
use anyhow::{Error, Result};
use rand::Rng;
use std::env;
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::thread;

use std::net::{IpAddr, Ipv4Addr};

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

//TODO, create a list of servers that connecto master, ping them all to see if alive, if yes add to a list. Send set to each item ont he list
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
                                           
                                                
        let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).await?;
        let cache = Arc::new(Mutex::new(Cache::new(CACHE_SIZE)));

        if PORT == "6379" {
            println!("Master Server Started");

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
                            println!("Received response from master");
                            let mut cache = cache.lock().unwrap();

                            // Deserialize the received bytes into a RESPMessage
                            let (response, _) = RESPMessage::deserialize(&buffer); //read response

                            let response_str = response.pack_string(); //convert from RESP to string

                            //remove from result
                            let result = response_str
                                .map(|value| value.to_string())
                                .unwrap_or_else(|err| format!("Error: {:?}", err));


                            let response_array: Vec<&str> = result.split(" ").collect(); //convert string to array

                            for i in 0..response_array.len() / 2 {
                                //loop array
                                if i == 0 {
                                    cache.set(
                                        response_array[i].to_string(),
                                        response_array[3].to_string(),
                                        Some(1000),
                                    ); //set to cache
                                } else {
                                    cache.set(
                                        response_array[i].to_string(),
                                        response_array[i +3 ].to_string(),
                                        Some(1000),
                                    );
                                }
                            }

                            let save = cache.get_key();
                            let (mut save_k, save_v) = save;
                            println!("Saved Values in Replica are:");
                            for i in 0..save_k.len(){
                                println!("Key {:?} -> Value {:?}", save_k[i] , save_v[i]);

                            }

                            println!("SYNC complete");
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
                            Some(value) => {
                                println!("Got value: {:?}", value);
                                RESPMessage::BulkString(value)
                            },
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
                            println!("Setting key: {:?} to value: {:?}", key, value);
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
                    // Acquire the lock on the cache and retrieve all keys
                    let mut resp = cache.lock().unwrap().get_key(); //get keys and values from cache
                    let (mut cache_k, cache_v) = resp; //returns to arrays

                    println!("Master Values {:?}, {:?}", cache_k, cache_v);

                    cache_k.extend(cache_v); //combine to 1 array
                    let mut joined_str = String::new();

                    for i in 0..cache_k.len() {
                        joined_str = cache_k.join(" ");
                        RESPMessage::SimpleString(cache_k[i].to_string());
                    }

                    RESPMessage::BulkString(joined_str)
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
