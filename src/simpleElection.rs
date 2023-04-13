use crate::{resp::RESPMessage};
use std::{collections::HashMap, process};
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::time::Duration;
use std:: thread;
use std::process::{Command, Stdio};
use regex::Regex;

// continuously ping the leader and start an election when timout occures
pub fn ping_leader(ports: &[String]) {
    loop {
        let response = send_ping_message_to_leader();
        let trimmed_response = response.trim_start_matches('+').trim();

        if trimmed_response != "PONG" {
            start_election(ports);
            thread::sleep(Duration::from_secs(10));
        } else {
            println!("PONG from leader recieved");
            thread::sleep(Duration::from_secs(10));
        } 
    }
}

// a simple election algorithm that elects the server with the highest process id
fn start_election(ports: &[String]) {
    let mut server_ids = HashMap::new();
    // loop over the secondary server ports and get their server Ids
    for port in ports {
        // send get server id message a save response
        let response = send_get_server_id_message(port.to_string());
        // match response with process ID regex (if not a match the server at that port is not alive)
        let reg = Regex::new(r"^\+\d+\r\n").unwrap();

        match reg.is_match(&response) {
            true => {
                // trim response and convert to i32
                let trimmed_response = response.trim_start_matches('+').trim();
                let server_id = trimmed_response.parse::<i32>().unwrap();

                // store server id as key in hash map
                server_ids.insert(server_id, port.to_string());
            },
            false => {
                println!("No server alive at port {}", port);
            }
        }
    }
    
    // loop over the server IDs and find the server with the largest I
    let mut max_id = std::process::id() as i32;
    for (&server_id, port) in server_ids.iter() {
        if server_id > max_id {
            max_id = server_id;
        }
    }
    // get the port of the server with max_id
    // if its not found in the map that means this server has the highest id
    match server_ids.get(&max_id) {
        Some(value) => {
            println!("server at port {} has the max id of {}", value, max_id);
            send_set_leader_message(value.to_string());
        },
        None => {
            println!("Initiating server has highest id of {}. Becoming leader...", max_id);
            let mut child = Command::new(std::env::args().next().unwrap())
                .arg("6379")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start new instance of the program");
            process::exit(0);
        },
    }
}

// request server id from a server
fn send_get_server_id_message(port: String) -> String {
    let addr = "localhost:".to_string() + "" + &port;
    let mut stream = match TcpStream::connect(addr) {
        Ok(stream) => stream,
        Err(e) => {
            return "NOTALIVE".to_string();
        }
    };

    stream.set_read_timeout(Some(Duration::from_secs(1)));

    let bulk_message: RESPMessage = RESPMessage::BulkString("GETSERVERID".to_string());

    let array = RESPMessage::Array(vec![
        bulk_message
    ]);

    let serialized = array.serialize();
    
    match stream.write_all(&serialized) {
        Ok(_) => {},
        Err(e) => println!("Error: {}", e),
        _ => println!("Default"),
    }
    let mut response = String::new();
    match stream.read_to_string(&mut response) {
        Ok(_) => println!("{}", response),
        Err(e) => {},
        _ => println!("Default case"),
    }
    response
}

// send message to notify a backup it shuld become the leader
fn send_set_leader_message(port: String) -> String {
    let addr = "localhost:".to_string() + "" + &port;
    let mut stream = match TcpStream::connect(addr) {
        Ok(stream) => stream,
        Err(e) => {
            return e.to_string();
        }
    };

    stream.set_read_timeout(Some(Duration::from_secs(1)));

    let bulk_message: RESPMessage = RESPMessage::BulkString("SETLEADER".to_string());

    let array = RESPMessage::Array(vec![
        bulk_message
    ]);

    let serialized = array.serialize();
    
    match stream.write_all(&serialized) {
        Ok(_) => println!("Write to stream OK"),
        Err(e) => println!("Error: {}", e),
        _ => println!("Default"),
    }
    let mut response = String::new();
    match stream.read_to_string(&mut response) {
        Ok(_) => println!("{}", response),
        Err(e) => {},
        _ => println!("Default case"),
    }
    response  
}

// send ping message to leader (leader always on port 6379)
fn send_ping_message_to_leader() -> String {
    let mut stream = match TcpStream::connect("localhost:6379") {
        Ok(stream) => stream,
        Err(e) => {
            return e.to_string();
        }
    };

    stream.set_read_timeout(Some(Duration::from_secs(1)));

    let bulk_message: RESPMessage = RESPMessage::BulkString("PING".to_string());

    let array = RESPMessage::Array(vec![
        bulk_message
    ]);

    let serialized = array.serialize();
    
    match stream.write_all(&serialized) {
        Ok(_) => println!("Write to stream OK"),
        Err(e) => println!("Error: {}", e),
        _ => println!("Default"),
    }
    let mut response = String::new();
    match stream.read_to_string(&mut response) {
        Ok(_) => println!("{}", response),
        Err(e) => {},
        _ => println!("Default case"),
    }
    response 
}