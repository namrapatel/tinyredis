use std::{net::{TcpListener, TcpStream}, io::{Read,Write}};

const MESSAGE_SIZE: usize = 1024;

fn handle_connection(mut stream: TcpStream) {
    loop {
        println!("hello");
        let mut buffer = [0; MESSAGE_SIZE];
        let read_data = stream.read(&mut buffer);

        println!("Received: {:?}", buffer);

        stream.write_all(b"Hello from Rust!\n").unwrap();
    }
}

fn main() -> std::io::Result<()> {
    println!("here");
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        handle_connection(stream?);
    }

    Ok(())
}