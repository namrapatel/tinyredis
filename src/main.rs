use std::{net::{TcpListener, TcpStream}, io::{Read,Write}};
use std::str;

const MESSAGE_SIZE: usize = 512;

fn handle_connection(mut stream: TcpStream) {
    println!("Handle connection called");
    let mut buffer = [0; MESSAGE_SIZE];
    _ = stream.read(&mut buffer);
    let message =  str::from_utf8(&buffer).unwrap();

    println!("{}", {message});
    let string = "+PONG\r\n";
    let reply = str::as_bytes(&string);
    _ = stream.write(&reply);

}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        handle_connection(stream?);
    }

    Ok(())
}