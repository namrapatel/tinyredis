use std::str;
use anyhow::{Result, Error};

const CRLF: &[u8] = b"\r\n";

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum RESPMessage {
    SimpleString(String),
    Error(String),
    Integer(u64),
    BulkString(String),
    Array(Vec<RESPMessage>),
    Null
}

fn start_to_cflf(bytes: &[u8]) -> usize {
    let mut index = 0;
    for byte in bytes {
        if *byte == b'\r' {
            break;
        }
        index += 1;
    }
    index
}

impl RESPMessage {
    // Function that encodes data into the respective RESP format
    pub fn serialize(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];
        match self {
            Self::SimpleString(s) => {
                result.push(b'+');
                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Error(s) => {
                result.push(b'-');
                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Integer(i) => {
                result.push(b':');
                let mut integer_bytes: Vec<u8> = i.to_string().as_bytes().to_owned();
                result.append(&mut integer_bytes);
                result.append(&mut CRLF.to_owned());
            }
            Self::BulkString(s) => {
                result.push(b'$');

                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                let mut length_bytes: Vec<u8> = s.len().to_string().as_bytes().to_owned();

                result.append(&mut length_bytes);
                result.append(&mut CRLF.to_owned());
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Array(a) => {
                result.push(b'*');

                // Get the length of the array
                let mut length_bytes = a.len().to_string().as_bytes().to_owned();
                result.append(&mut length_bytes);
                result.append(&mut CRLF.to_owned());

                // Handle each element of the array
                for element in a {
                    let mut element_bytes = element.serialize();
                    result.append(&mut element_bytes);
                }
            },
            Self::Null => {
                result.push(b'$');
                // add b'-1' to the result
                let mut null_bytes = b"-1".to_vec();
                result.append(&mut null_bytes);
                result.append(&mut CRLF.to_owned());
            }
        }
        result
    }

    // Function that decodes data from the respective RESP format
    pub fn deserialize(bytes: &[u8]) -> (Self, usize) {
        match bytes[0] {
            b'+' => {
                let len: usize = start_to_cflf(&bytes[1..]);
                return (
                    Self::SimpleString(str::from_utf8(&bytes[1..len]).unwrap().to_string()),
                    len + 3, // +3 for the CRLF and the + 
                );
            },
            b'-' => {
                let len: usize = start_to_cflf(&bytes[1..]);
                return (
                    Self::Error(str::from_utf8(&bytes[1..len]).unwrap().to_string()),
                    len + 3, // +3 for the CRLF and the - 
                );
            },
            b':' => {
                let len: usize = start_to_cflf(&bytes[1..]);
                return (
                    Self::Integer(str::from_utf8(&bytes[1..len]).unwrap().parse().unwrap()),
                    len + 3, // +3 for the CRLF and the : 
                );
            },
            b'$' => {
                // Reads the length from the first byte of the serialized message
                let len: usize = start_to_cflf(&bytes[1..]);
                // Determines the length of the actual string message within the serialized message
                let length: i32  = str::from_utf8(&bytes[1..len]).unwrap().parse().unwrap(); 
                
                // Handle Null BulkString 
                if length == -1 {
                    return (Self::Null, len + 3);
                }

                // Handle Empty BulkString
                if length == 0 {
                    return (Self::BulkString("".to_string()), len + 3);
                }

                return (
                    Self::BulkString(str::from_utf8(&bytes[len + 3..len + 3 + length as usize]).unwrap().to_string()),
                    len + 3 + length as usize + 3, // +3 for the CRLF and the $, +3 for the CRLF and the length
                );
            },
            // TODO: potentially buggy
            b'*' => {
                // Reads the length from the first byte of the serialized message
                let len: usize = start_to_cflf(&bytes[1..]);
                // Determines the length of the actual string message within the serialized message
                let length_str = str::from_utf8(&bytes[1..len]).unwrap();
                let length: i32 = if length_str.is_empty() { 0 } else { length_str.parse().unwrap() };  

                let mut index = len + 3;
                let mut array: Vec<RESPMessage> = vec![];
                for _ in 0..length {
                    let (message, len) = Self::deserialize(&bytes[index..]);
                    array.push(message);
                    index += len;
                }
                return (Self::Array(array), index);
            },
            _ => (Self::Error("Invalid RESP message type".to_string()), 0),
        }
    }

    // This is the function that we will use to convert the RESPMessage to a String
    pub fn pack_string(&self) -> Result<&str> {
        match self {
            Self::SimpleString(s) | Self::BulkString(s) => Ok(s),
            _ => Err(Error::msg("Trying to decode non-string")),
        }
    }

    pub fn to_command(&self) -> Result<(String, Vec<RESPMessage>)> {
        match self {
            RESPMessage::Array(elements) => {
                let (first, rest) = elements.split_first().unwrap();
                let first_str = match first {
                    RESPMessage::BulkString(s) => s.clone(),
                    _ => return Err(Error::msg("Invalid RESP message: not a BulkString.")),
                };
                let remaining: Vec<RESPMessage> = rest.iter().cloned().collect();
                Ok((first_str, remaining))
            },
            _ => Err(Error::msg("not an array")),
        }
    }


}