use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use crate::{resp::RESPMessage};
use anyhow::{Result, Error};

const MESSAGE_SIZE: usize = 512;

pub struct Server {
    listener: TcpListener
}

impl Server {

    pub async fn run() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:6379").await?;

        loop {
            let incoming = listener.accept().await;

            match incoming {
                Ok((mut stream, addr)) => {
                    println!("Handling connection from: {}", addr);
                    tokio::spawn(async move {
                        Self::handle_connection(&mut stream).await.unwrap();
                    });
                },
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
    }

    async fn handle_connection(stream: &mut TcpStream) -> Result<()> {
        let mut buffer = [0; MESSAGE_SIZE];

        loop { 
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                println!("Closing connection.");
                break;
            }

            println!("Buffer contents: {:?}", String::from_utf8_lossy(&buffer));

            let (message, _) = RESPMessage::deserialize(&buffer);
            println!("Message recieved: {:?}", {message.clone()});
            println!("MADE IT");
            let (command, args) = message.to_command()?;
            println!("Here is the command: {}", {command.clone()});
            let response = match command.to_ascii_lowercase().as_ref() {
                "ping" => {
                    RESPMessage::SimpleString("PONG".to_string())
                },
                // "echo" => args.first().unwrap().clone(),
                _ => RESPMessage::Error("Error".to_string())
            };
            let serialized_response = RESPMessage::serialize(&response);
            stream.write_all(&serialized_response).await;
        }
        Ok(())
    }

}
