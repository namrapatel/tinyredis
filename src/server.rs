use std::{io::{Read,Write,Result}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use std::str;
use crate::{resp::RESPMessage};

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

            let (message, _) = RESPMessage::deserialize(&buffer);

            // println!("Message recieved: {:#?}", {message});

            // deserialize from resp -> ping
            // 
            match message {
                RESPMessage::Array(_) => {
                    let string = "+PONG\r\n";
                    let reply = str::as_bytes(&string);
                    _ = stream.write_all(&reply);
                },
                _ => { break }
            }   
            stream.write("+PONG\r\n".as_bytes()).await?;
        }
        Ok(())
    }

}
