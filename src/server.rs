use anyhow::{Result, Error};
use crate::{resp::RESPMessage, cache::{Cache, self}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};

const MESSAGE_SIZE: usize = 512;

pub struct Server {
    listener: TcpListener,
    cache: Cache
}

impl Server {

    pub async fn new() -> Result<Self, Error> {
        let listener = TcpListener::bind("127.0.0.1:6379").await?;
        let mut cache = Cache::new();
    
        Ok(Self { listener, cache })
    }
    

    pub async fn run(server: Server) -> Result<()> {

        loop {
            let incoming = server.listener.accept().await;
            match incoming {
                Ok((mut stream, addr)) => {
                    println!("Handling connection from: {}", addr);
                    tokio::spawn(async move {
                        Self::handle_connection(&mut stream, &mut server.cache).await.unwrap();
                    });
                },
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
    }

    async fn handle_connection(stream: &mut TcpStream, cache: &mut Cache) -> Result<()> {
        let mut buffer = [0; MESSAGE_SIZE];

        loop { 
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                println!("Closing connection.");
                break;
            }
            // println!("Buffer contents: {:?}", String::from_utf8_lossy(&buffer));

            let (message, _) = RESPMessage::deserialize(&buffer);
            // println!("Message recieved: {:?}", {message.clone()});

            let (command, args) = message.to_command()?;
            // println!("Here is the command: {}", {command.clone()});
            let response = match command.to_ascii_lowercase().as_ref() {
                "ping" => {
                    RESPMessage::SimpleString("PONG".to_string())
                },
                "echo" => args.first().unwrap().clone(),
                "set" => {
                    
                }
                _ => RESPMessage::Error("Error".to_string())
            };
            let serialized_response = RESPMessage::serialize(&response);
            stream.write_all(&serialized_response).await;
        }
        Ok(())
    }

}
