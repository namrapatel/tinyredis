use std::{io::{Read,Write,Result}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use std::str;

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
        loop { 
            let mut buffer = [0; MESSAGE_SIZE];
            _ = stream.read(&mut buffer).await?;
            
            let message =  str::from_utf8(&buffer).unwrap();
            // println!("Message recieved: {}", {message});
            match message {
                "*1\n$4\nPING\n" => {
                    let string = "+PONG\r\n";
                    let reply = str::as_bytes(&string);
                    _ = stream.write_all(&reply);
                },
                _ => { break }
            }   
        }
        Ok(())
    }

}
